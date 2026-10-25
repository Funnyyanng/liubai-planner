use planner_contracts::*;
use std::collections::{BTreeMap, BTreeSet};
pub fn validate(manifests: &[ModuleManifest]) -> Result<Vec<String>> {
    let mut map = BTreeMap::new();
    for m in manifests {
        if m.contract_version != CONTRACT_VERSION {
            return Err(AppError::new(
                "contract_mismatch",
                format!("{} 契约版本不兼容", m.id),
            ));
        }
        if map.insert(m.id.clone(), m).is_some() {
            return Err(AppError::new("duplicate_module", &m.id));
        }
    }
    let mut order = vec![];
    let mut visited = BTreeSet::new();
    let mut stack = BTreeSet::new();
    fn visit(
        id: &str,
        map: &BTreeMap<String, &ModuleManifest>,
        visited: &mut BTreeSet<String>,
        stack: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) -> Result<()> {
        if visited.contains(id) {
            return Ok(());
        }
        if !stack.insert(id.into()) {
            return Err(AppError::new(
                "dependency_cycle",
                format!("模块循环依赖: {id}"),
            ));
        }
        let m = map
            .get(id)
            .ok_or_else(|| AppError::new("missing_dependency", id))?;
        for d in &m.dependencies {
            visit(d, map, visited, stack, order)?
        }
        stack.remove(id);
        visited.insert(id.into());
        order.push(id.into());
        Ok(())
    }
    for id in map.keys() {
        visit(id, &map, &mut visited, &mut stack, &mut order)?
    }
    Ok(order)
}

pub type LifecycleHook = fn(&crate::UnitOfWork<'_>) -> Result<()>;
pub struct Registration {
    pub manifest: ModuleManifest,
    pub initialize: LifecycleHook,
    pub shutdown: LifecycleHook,
}
pub struct ModuleRuntime {
    registrations: Vec<Registration>,
    order: Vec<String>,
    pub health: Vec<ModuleHealth>,
}
impl ModuleRuntime {
    pub fn initialize(store: &crate::Store, registrations: Vec<Registration>) -> Result<Self> {
        let manifests = registrations
            .iter()
            .map(|r| r.manifest.clone())
            .collect::<Vec<_>>();
        let order = validate(&manifests)?;
        let mut health: Vec<ModuleHealth> = vec![];
        for id in &order {
            let r = registrations.iter().find(|r| &r.manifest.id == id).unwrap();
            let failed_dependency = r
                .manifest
                .dependencies
                .iter()
                .any(|d| health.iter().any(|h| &h.id == d && h.status != "ready"));
            let status = if failed_dependency {
                Err(AppError::new("module_dependency", "必需模块未就绪"))
            } else {
                store.read(r.initialize)
            };
            match status {
                Ok(()) => health.push(ModuleHealth {
                    id: id.clone(),
                    status: "ready".into(),
                    detail: "初始化完成，契约与迁移已验证".into(),
                }),
                Err(e) => {
                    if r.manifest.critical {
                        return Err(AppError::new(
                            "critical_module",
                            format!("{} 初始化失败：{}", id, e.message),
                        ));
                    }
                    health.push(ModuleHealth {
                        id: id.clone(),
                        status: "failed".into(),
                        detail: e.message,
                    });
                }
            }
        }
        Ok(Self {
            registrations,
            order,
            health,
        })
    }
    pub fn shutdown(&self, store: &crate::Store) {
        for id in self.order.iter().rev() {
            if self
                .health
                .iter()
                .any(|h| &h.id == id && h.status == "ready")
            {
                if let Some(r) = self.registrations.iter().find(|r| &r.manifest.id == id) {
                    let _ = store.read(r.shutdown);
                }
            }
        }
    }
    pub fn available(&self, id: &str) -> bool {
        self.health
            .iter()
            .any(|h| h.id == id && h.status == "ready")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn m(id: &str, deps: Vec<&str>) -> ModuleManifest {
        ModuleManifest {
            id: id.into(),
            version: "0.1.0".into(),
            contract_version: 1,
            dependencies: deps.into_iter().map(String::from).collect(),
            optional_capabilities: vec![],
            commands: vec![],
            queries: vec![],
            events: vec![],
            pages: vec![],
            critical: true,
            migration_version: 1,
        }
    }
    #[test]
    fn checks_graph() {
        assert!(validate(&[m("a", vec!["b"])]).is_err());
        assert!(validate(&[m("a", vec!["b"]), m("b", vec!["a"])]).is_err());
        assert_eq!(
            validate(&[m("a", vec!["b"]), m("b", vec![])]).unwrap(),
            vec!["b", "a"]
        );
        let mut bad = m("bad", vec![]);
        bad.contract_version = 2;
        assert!(validate(&[bad]).is_err());
    }
}
