# 新增独立模块

## 选择边界

新模块应拥有自己领域的数据和界面，只通过契约提供任务、约束或查询。不要让排程引擎了解训练动作、课程名称或具体研究步骤。

现有训练模块是参考：配置存于 `training_records`，通过 `contribute` 输出普通 Task、Allowed、MinimumGap、Quota；协调器将新任务交给任务模块，将约束交给排程器。关闭训练模块只停止安排，保留已完成记录。

## 接入步骤

1. 建立 `modules/<name>` Cargo 包，依赖公共契约及必要框架能力；业务实现仅从 lib.rs 导出稳定接口，内部帮助函数保持私有。
2. 编写模块清单并加入根注册表，声明依赖、能力、公开命令／查询／事件、页面及迁移版本。运行边界检查验证清单一致。
3. 通过 `UnitOfWork.repo("<name>")` 访问自身数据。不得直接操作 SQLite、读取其他模块命名空间，或把外部 API 调用放进业务代码。
4. 在协调器组合入口注册初始化／关闭回调。纯计算模块不需要数据库资源。
5. 需要参与排程时，输出 `WorkDemand`、`Constraint` 等已支持契约；新增算法只替换 scheduler 内部实现，保留 propose／validate／recommend 的公共行为。
6. 为需要用户交互的模块建立 `src/modules/<name>` 页面；通过 `src/shell/registry.ts` 注册页面，复用外壳提供的命令适配，不从页面访问 SQLite 或系统能力。
7. 为独立逻辑写快照或模拟 ModelTransport 测试；对持久化流程使用隔离临时数据库，验证幂等、失败回滚与恢复。

## 契约变更

- 兼容字段需要 serde 默认值，并同步 TypeScript 类型和测试。
- 非兼容字段、约束语义或命令格式变更必须提升契约版本，更新全部调用方。
- 数据模式升级必须新增迁移实现及测试，再提高模块迁移版本；v1 只注册初始建表迁移，未知升级明确报错。
- 不能用“忽略未知约束”换取表面兼容。缺少能力时应显示无法安排的原因。
- 模块版本与数据迁移版本分别维护。第一版统一应用发布，不承诺模块热替换或旧二进制读取新数据库。

## 验证命令

```sh
npm run check:boundaries
npm run build
npm test
node scripts/rust.mjs test -p planner-training
node scripts/rust.mjs test --workspace
node scripts/rust.mjs clippy --workspace --all-targets -- -D warnings
```

CI 使用 macOS，执行前端构建、边界检查、测试、Rust 格式／静态检查以及桌面打包。
