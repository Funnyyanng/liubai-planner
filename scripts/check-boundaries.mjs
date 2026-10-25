import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
const manifests = JSON.parse(readFileSync("modules.json", "utf8"));
const map = new Map(manifests.map((m) => [m.id, m]));
const errors = [];
const seen = new Set();
const visiting = new Set();
function visit(id) {
  if (seen.has(id)) return;
  if (visiting.has(id)) {
    errors.push(`模块依赖循环 ${id}`);
    return;
  }
  const m = map.get(id);
  if (!m) {
    errors.push(`缺少模块 ${id}`);
    return;
  }
  visiting.add(id);
  for (const d of m.dependencies) visit(d);
  visiting.delete(id);
  seen.add(id);
}
function files(path) {
  return readdirSync(path, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? files(join(path, e.name)) : [join(path, e.name)],
  );
}
for (const m of manifests) {
  visit(m.id);
  if (m.contractVersion !== 1) errors.push(`${m.id}: 不兼容契约`);
  const local = JSON.parse(
    readFileSync(`modules/${m.id}/manifest.json`, "utf8"),
  );
  if (JSON.stringify(local) !== JSON.stringify(m))
    errors.push(`${m.id}: 清单与注册表不同步`);
  const cargo = readFileSync(`modules/${m.id}/Cargo.toml`, "utf8");
  const allowed = new Set([
    ...m.dependencies,
    ...m.optionalCapabilities,
    "contracts",
    "framework",
  ]);
  for (const hit of cargo.matchAll(/planner-([a-z_]+)\s*=/g))
    if (!allowed.has(hit[1])) errors.push(`${m.id}: 未声明依赖 ${hit[1]}`);
  for (const path of files(`modules/${m.id}/src`).filter((p) =>
    p.endsWith(".rs"),
  )) {
    const text = readFileSync(path, "utf8");
    for (const hit of text.matchAll(/\.repo\("([^"]+)"\)/g))
      if (hit[1] !== m.id) errors.push(`${path}: 跨模块数据访问 ${hit[1]}`);
    if (/rusqlite|reqwest|std::process|std::fs|keyring/.test(text))
      errors.push(`${path}: 业务模块绕过基础框架`);
    if (/planner_\w+::(?:internal|repository|storage|private)::/.test(text))
      errors.push(`${path}: 私有实现依赖`);
  }
}
const contracts = readFileSync("crates/contracts/Cargo.toml", "utf8");
if (/planner-(?!contracts)/.test(contracts))
  errors.push("公共契约依赖业务实现");
if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}
console.log(
  `模块边界检查通过：${manifests.length} 个模块，契约 v1，依赖无环。`,
);
