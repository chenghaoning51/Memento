# ADR-0008：信件的开启时刻存 UTC，状态在读取时推进

- 状态：Accepted
- 日期：2026-10-17

## 背景

M4 要把未来信件从产品规则变成可运行的功能。产品文档已经定了三件事：开启时刻取所选本地日期的 00:00:00 并按创建时区换算成 UTC；锁定、到期、已打开三种状态在启动与进入页面时按当前 UTC 重算；到期后原信冻结，只能追加一条独立保存的回应。

需要落地的判断有三处：开启时刻存什么、时区放在哪里、状态列与当前时刻的关系怎么处理。这三处都会进入数据库格式，改动成本高，因此先记一条 ADR。

## 决定

- 迁移 v5 建 `letters` 表：`id`、`title`、`content_json`、`content_text`、`unlock_at_utc`、`unlock_timezone`、`status`、`reply_json`、`reply_text`、`opened_at_utc`、`created_at_utc`、`updated_at_utc`，加 `letters_by_unlock` 索引。
- `unlock_at_utc` 存**换算后的 UTC 时刻**，不再存“日期 + 时刻”两个字段；`unlock_timezone` 存写信时的 IANA 时区名，只用于把同一个时刻解释回用户原本的意图。
- 时区换算在前端完成（`Intl.DateTimeFormat().resolvedOptions().timeZone` 与本地 `Date` 构造），Rust 不引入时区库，也不在读取时重算时刻。
- `status` 列只在写入时写 `locked` 或 `unlocked`；读取时在内存里按当前 UTC 推进到 `unlocked`，不写库。只有真正打开时才写 `opened` 与 `opened_at_utc`，且 `opened_at_utc` 一旦有值就不再覆盖。
- 到期后 `update_letter` 拒绝改原文，判断同时看状态列与当前时刻（只看到期但未打开的信，状态列仍是 `locked`）。回应走单独的 `save_letter_reply`，写 `reply_json` / `reply_text`，不碰原信字段。
- 回应沿用正文的 version 2 文档格式存 `reply_json`，同时存纯文本 `reply_text`，方便以后把回应升级成富文本而不用迁移。

## 原因

- 存 UTC 时刻而不是本地日期：本地日期在换时区后会指向不同的瞬间。存时刻之后，同一封信在任何时区都指向同一个物理时刻，A14 的“切换时区不漂移”才有定义。
- 时区名单独存：只有 UTC 时刻时，用户看到“2026年11月1日 00:00”会疑惑是哪里的午夜；存了写信时区，列表与顶栏都能按原本的意图显示，而不受当前系统时区影响。
- 换算放前端：本机时区与夏令时规则由浏览器/系统提供，Rust 侧要自己算就得引入 `chrono-tz` 一类依赖与随系统更新的时区库，收益不足。
- 状态在读取时推进：产品明确“不依赖后台定时器”，应用关着的时候到期也要在重开后立即正确。把推进放在读取路径，进程内不需要任何定时任务。
- `status` 列不随读取改写：每次进页面都 UPDATE 会把读操作变成写操作，WAL 下没有收益，还会让 `updated_at_utc` 失去“用户改过信”的含义。真正的状态变化只有“打开”一次，值得落库。
- 打开时间只写一次：`mark_opened` 带 `opened_at_utc IS NULL` 条件，重复打开不会把“第一次读到”改成“最近一次读到”。

## 备选方案

- **存本地日期 + 时区，读时换算**：换时区后同一封信的开启瞬间会变，与产品规则冲突。
- **只存 UTC 时刻，不存时区**：实现更简单，但跨时区打开时无法解释“为什么是 00:00”，A13 的“原始时区可以解释开启意图”无法满足。
- **每次读取都把状态写回数据库**：状态列始终准确，但把只读操作变成写事务，且会让 `updated_at_utc` 无法表达用户行为。
- **用后台定时器在到期瞬间切换状态**：应用关着时不生效，且引入一条常驻任务；产品文档已排除。
- **到期后允许编辑原信**：与“写给未来”的约定冲突，用户无法判断读到的还是不是当初写下的内容。

## 影响

- 迁移 v5：新增 `letters` 表与 `letters_by_unlock` 索引；`PRAGMA user_version` 变为 5。
- `src-tauri/src/letters/` 新增 `mod.rs`（三态常量与 `status_at`）与 `repository.rs`（列表、查找、插入、更新、删除、`mark_opened`、`save_reply`）。
- 新增七个 Tauri 命令：`list_letters`、`get_letter`、`create_letter`、`update_letter`、`delete_letter`、`open_letter`、`save_letter_reply`，命令总数变为二十四个。
- 前端新增 `src/features/letters/LettersPage.tsx`、`src/lib/letterApi.ts`、`src/lib/letterContent.ts`、`src/lib/unlockTime.ts` 与 `src/types/letter.ts`；导航“未来信箱”从占位变为真实页面。
- 锁定是展示限制而不是加密：数据库里的正文是明文，产品文案不得写成“已加密”或“安全封存”。这一点沿用[信件产品文档](../product/letters.md)。
- M5 的备份清单必须显式包含 `letters` 表（含 `reply_json` / `reply_text` / `opened_at_utc`），否则恢复后信件与回应会丢失。
- 若将来增加到期提醒或应用锁，需要在读取推进之外再加一条机制，届时另写 ADR。
