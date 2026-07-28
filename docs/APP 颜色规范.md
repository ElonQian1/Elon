# APP UI 设计规范

最后更新：2026-07-24

本文件替代旧版“APP 颜色规范”，作为一龙 APP 的完整 UI 设计规范。文件名保留历史路径，方便现有 AI 路由、脚本和说明继续定位；内容以本版为准。任何 APP 页面、主题、颜色、按钮、卡片、列表、底部导航、弹窗、状态标签或项目空间 UI 调整，都必须先遵守本文件；除非任务本身是更新本规范。

本规范基于 3 张项目管理参考图提炼，核心风格定义为 **Elon 墨黑项目工作台**：大面积纯黑背景，深灰承载结构，白色胶囊承担主要动作，红绿小圆点表达状态。整体要像一个安静、专业、可长期使用的移动端项目管理工具，不做营销页感、插画感或彩色装饰感。

颜色部分必须按截图采样值实现，不允许用“相近色”“更亮一点”“差不多的灰”替代。比如规范中底部导航是 `#1A1A1A`，实现时就必须使用 `#1A1A1A`，不能改成 `#202020` 或 `#222222`。

## 设计原则

- 纯黑优先：我的项目列表、项目广场页面、详情页顶部 chrome、项目空间内容画布统一使用 `#000000`。
- 结构克制：用深灰卡片、分区条、圆角和留白建立层级，不依赖彩色边框。
- 白色动作：主要按钮统一为白底黑字胶囊，例如“进入空间”“下载APK”“AI 会话”。
- 状态点缀：绿色和红色只用于状态点、在线、通过、需审批、异常，不作为大面积按钮底色。
- 大字导航：一级页面使用粗体大标题和短下划线表达选中，不使用传统顶部 Tab 条边框。
- 胶囊筛选：搜索框、筛选项、分段控件均为圆角胶囊，选中态用深灰填充。
- 信息密度适中：列表页保持扫描效率，卡片页允许更强留白和更大圆角。
- 原生工具感：图标使用线性轮廓，菜单使用白色浮层，底部导航采用固定的悬浮胶囊，稳定、低干扰。
- 不做渐变、阴影光效、蓝紫主色、彩色大按钮、装饰圆点背景。

## 颜色系统

### 截图采样颜色 Token

| 用途 | 设计 Token | Android 建议名 | Web CSS 变量 | HEX | 来源 |
|---|---|---|---|---:|---|
| 我的项目列表背景 | `color.bg.mine` | `elon_bg_mine` | `--bg-mine` | `#000000` | 图 1 页面背景 |
| 项目空间内容画布 | `color.bg.canvas` | `elon_bg_canvas` | `--bg-canvas` | `#000000` | 图 3 内容区 |
| 项目广场页面背景 | `color.bg.plaza` | `elon_bg_plaza` | `--bg-plaza` | `#000000` | 图 2 页面背景，按图 1 纯黑背景统一 |
| 详情页顶部背景 | `color.bg.chrome` | `elon_bg_chrome` | `--bg-chrome` | `#000000` | 图 3 顶部区域，按纯黑背景统一 |
| 底部导航背景 | `color.nav.bg` | `elon_nav_bg` | `--nav-bg` | `#1A1A1A` | 2026-06-25 用户要求与上方提示条统一 |
| 列表选中胶囊 | `color.surface.segment.selected` | `elon_segment_selected` | `--segment-selected` | `#1A1A1A` | 图 1 “独立”、图 2 “全部” |
| 项目广场卡片主体 | `color.surface.card` | `elon_surface_card` | `--panel` | `#1A1A1A` | 图 2 卡片主体 |
| 卡片头部 / 公告容器 | `color.surface.header` | `elon_surface_header` | `--panel-header` | `#1F2023` | 图 2 卡片头部、图 3 公告 |
| 搜索框背景 | `color.surface.search` | `elon_surface_search` | `--search-bg` | `#272727` | 图 2 搜索框 |
| 圆形悬浮暗按钮 | `color.surface.float` | `elon_surface_float` | `--float` | `#212121` | 图 1 顶部加号、图 3 内容加号 |
| 圆形按钮描边 | `color.border.float` | `elon_border_float` | `--float-border` | `#4D4D4D` | 图 1 顶部加号边框 |
| 卡片内分割线 | `color.divider.card` | `elon_divider_card` | `--divider-card` | `#6D6E6F` | 图 2 卡片内横线 |
| 主操作按钮 / 菜单 / 封面占位 | `color.surface.inverse` | `elon_surface_inverse` | `--surface-inverse` | `#FFFFFF` | 图 1 封面、图 2 按钮、图 3 菜单 |
| 一级 Tab / 广场卡片标题 / 状态文字 | `color.text.primary` | `elon_text_primary` | `--ink` | `#D9D9D9` | 图 1/2 大标题、图 2 卡片标题 |
| 我的项目列表项标题 | `color.text.list.title` | `elon_text_list_title` | `--ink-list-title` | `#FFFFFF` | 图 1 “项目名称” |
| 好友页列表摘要 / 时间 / 项目标识 | `color.text.list.preview` | `elon_text_list_preview` | `--ink-list-preview` | `#606060` | 微信会话列表摘要采样 |
| 详情页居中标题 | `color.text.detail.title` | `elon_text_detail_title` | `--ink-detail-title` | `#F4F5FB` | 图 3 顶部“魔王” |
| 正文元信息 | `color.text.secondary` | `elon_text_secondary` | `--ink-muted` | `#B8B8B8` | 图 2 创建者、简介 |
| 搜索占位 / 空状态 / 公告正文 | `color.text.placeholder` | `elon_text_placeholder` | `--ink-placeholder` | `#AFAFAF` | 图 2 搜索占位、图 3 空状态 |
| 底部导航图标文字 | `color.text.nav` | `elon_text_nav` | `--ink-nav` | `#D6D6D6` | 图 1/2 底部导航 |
| 弱提示 / 次弱信息 | `color.text.quiet` | `elon_text_quiet` | `--ink-quiet` | `#777777` | 图 1 弱文字边缘采样 |
| 白色按钮文字 | `color.text.inverse` | `elon_text_inverse` | `--ink-inverse` | `#000000` | 白色按钮上的纯黑文字 |
| 弹出菜单文字 | `color.text.menu` | `elon_text_menu` | `--menu-ink` | `#2F3136` | 图 3 白色菜单文字 |
| 顶部成员图标 | `color.icon.member` | `elon_icon_member` | `--icon-member` | `#A5AFBD` | 图 3 右上成员图标 |
| 加号 / 主线性图标 | `color.icon.primary` | `elon_icon_primary` | `--icon-primary` | `#D9D9D9` | 图 3 内容加号 |
| 顶部加号图标 | `color.icon.add.top` | `elon_icon_add_top` | `--icon-add-top` | `#D3D3D3` | 图 1 顶部加号 |
| 底部主菜单抽屉展开图标 | `color.icon.menu.open` | `ic_bottom_nav_menu_active` | `__BOTTOM_NAV_MENU_ACTIVE_PNG_B64__` | `#5DA6FF` | 2026-07-24 用户提供三横杠素材 |
| 成功 / 无需审批 / 可安装状态点 | `color.status.success` | `elon_status_success` | `--success` | `#58BE6A` | 图 2 绿色状态点 |
| 项目状态 / 好友页下拉筛选项目进度 | `color.status.project` | `elon_status_project` | `--pull-filter-project` | `#F2C94C` | 好友页项目角标与项目筛选进度 |
| 需审批 / 异常状态点 | `color.status.danger` | `elon_status_danger` | `--danger` | `#E62129` | 图 2 红色状态点 |
| 项目空间商店详情背景 | `color.store.detail.bg` | `elon_store_detail_bg` | `--store-detail-bg` | `#131313` | 2026-06-29 Google Play 详情页参考图 |
| 项目空间商店详情主文字 | `color.store.detail.text.primary` | `elon_store_detail_text_primary` | `--store-detail-ink` | `#E3E3E3` | 2026-06-29 Google Play 详情页参考图 |
| 项目空间商店详情次文字 | `color.store.detail.text.secondary` | `elon_store_detail_text_secondary` | `--store-detail-muted` | `#C6C6C6` | 2026-06-29 Google Play 详情页参考图 |
| 项目空间商店详情链接蓝 | `color.store.detail.link` | `elon_store_detail_link` | `--store-detail-link` | `#A8C7FA` | 2026-06-29 Google Play 详情页参考图 |
| 项目空间商店详情分隔线 | `color.store.detail.divider` | `elon_store_detail_divider` | `--store-detail-divider` | `#444444` | 2026-06-29 Google Play 详情页参考图 |
| 项目空间商店详情安装按钮 | `color.store.detail.button` | `elon_store_detail_button` | `--store-detail-button` | `#AEC6F6` | 2026-06-29 Google Play 详情页参考图 |
| 项目空间商店详情安装按钮文字 | `color.store.detail.button.text` | `elon_store_detail_button_text` | `--store-detail-button-ink` | `#182E63` | 2026-06-29 Google Play 详情页参考图 |

### 颜色使用规则

- 图 1 “我的项目”列表页根背景必须是 `#000000`。
- 图 2 “项目广场”页面根背景必须与图 1 一致，使用 `#000000`。
- 图 3 项目空间顶部 chrome 和内容画布都必须使用 `#000000`。
- 底部导航使用 `#1A1A1A`；顶部圆形加号、内容区圆形加号继续使用 `#212121`。
- 项目广场卡片主体和选中筛选胶囊统一使用 `#1A1A1A`。
- 项目广场卡片头部、项目空间公告容器统一使用 `#1F2023`。
- 搜索框只能使用 `#272727`，不能复用 `#1A1A1A` 或 `#212121`。
- 白色 `#FFFFFF` 只用于主操作按钮、弹出菜单、白色封面占位。
- 绿色 `#58BE6A` 仅用于成功状态点、在线点、可安装、无需审批、完成进度，以及好友页下拉切到好友筛选的进度提示。
- 黄色 `#F2C94C` 仅用于项目角标和好友页下拉切到项目筛选的进度提示，不作为大面积装饰色。
- 红色 `#E62129` 仅用于需审批、失败、危险状态点，不做整块红色警告卡。
- 分割线使用图 2 卡片内采样色 `#6D6E6F`，长度不要贯穿整卡。
- 禁止新增相近黑灰色阶；确实需要新增时，必须从新参考图中采样并补充本表。

## 字体与层级

字体使用系统默认中文字体：Android 优先 `sans-serif` / `sans-serif-medium`，Web 优先系统字体栈。不要设置负字距，不使用花体、衬线体或品牌展示字体。

APP 字体尺寸以当前 **好友页面** 和 **我的页面** 的实际 UI 为标准。其它页面不再单独建立字号规范；如果需要新增字体层级，必须先回到好友页 / 我的页验证是否已有可复用层级。

| Token | 场景 | textSize | 字重 | 来源 |
|---|---|---:|---:|---|
| `font.profile.name` | 我的页个人卡片昵称 / 头像文字 | `24sp` | `700` | `UserProfileViews` |
| `font.profile.row.title` | 我的页个人资料行标题 | `17sp` | `400` | `UserProfileViews.row` |
| `font.page.title` | 好友页、我的页顶部普通标题 / 好友会话标题 / 我的页功能入口 | `16sp` | `400` | `activity_main.xml` |
| `font.profile.value` | 我的页个人资料行右侧值 / 主要辅助值 | `16sp` | `400` | `UserProfileViews.row` |
| `font.profile.info` | 我的页工作台说明 / 账号文本 | `15sp` | `400` | `activity_main.xml`、`UserProfileViews` |
| `font.bottom.nav` | 旧版底部导航文字（新悬浮导航仅显示图标） | `14sp` | `400` | `MainTabText` |
| `font.list.secondary` | 好友会话摘要 / 我的页签名与次级正文 | `13sp` | `400` | `activity_main.xml`、`UserProfileViews` |
| `font.meta.small` | 好友页时间 / 我的页版本号 / 辅助提示 | `12sp` | `400` | `activity_main.xml` |

文字颜色规则：

- 主标题、好友会话标题、我的页功能入口使用 `color.text.primary`。
- 好友页列表的会话摘要、项目摘要、右侧时间和项目标识使用 `color.text.list.preview`，用于和好友名称/项目名称拉开层级。
- 我的页账号、签名、版本号使用 `color.text.secondary`、`color.text.placeholder` 或 `color.text.quiet`。
- 白色按钮内文字使用 `color.text.inverse`，可沿用 `font.page.title` 并加粗。
- 禁止用绿色表达普通标题或普通链接，绿色只表达状态或少数明确状态入口。

## 布局栅格

- 基础单位为 `4dp`，常用间距必须落在 `8dp`、`12dp`、`16dp`、`20dp`、`24dp`、`32dp`。
- 页面左右安全边距：列表页 `24dp`，卡片页 `16dp`，详情页 `20dp`。
- 顶部内容距离状态栏：无系统标题栏页面使用 `36dp` 到 `48dp`。
- 一级导航到下一内容区：`48dp` 到 `64dp`，营造黑场留白。
- 卡片之间距离：`12dp` 到 `16dp`。
- 列表项内部图文间距：头像和文字 `18dp` 到 `20dp`。
- 底部悬浮导航可视高度：`56dp`；外层基准高度 `72dp`，由顶部 `8dp`、组件 `56dp`、底部 `8dp` 组成，系统安全区另行叠加。
- 所有可点击控件最小触控尺寸：`48dp x 48dp`。

## 按图复刻视觉验收

当用户提供截图、红框标注、手绘图或明确要求“按比例复刻 UI 设计”时，该任务必须按图稿复刻处理，完成前必须做视觉验收，不允许只凭感觉交付。

- 排版对齐：标题、正文、按钮、图标、卡片边缘必须和参考图的主轴、边距、基线保持一致；不得出现按钮漂浮、偏上/偏下、左右不齐。
- 板块比例：参考图中的模块高度、宽度、圆角、图标容器和留白比例必须按屏幕尺寸等比换算；不得为了塞内容随意拉伸或缩小局部元素。
- 字体规范：字号、字重、行高和颜色必须复用本规范已有层级；同一区块内标题、正文、辅助信息不能混用不成体系的字号。
- 间距合理：外边距、内边距、元素间距必须落在布局栅格内，并与参考图视觉节奏一致；按钮与文字、卡片与卡片之间不能拥挤或松散。
- 触控与容器：所有可点控件触控区不小于 `48dp x 48dp`，但视觉容器大小必须与参考图协调；不能只满足可点尺寸而破坏对齐。
- 完成前自查：Android 必须尽量通过真机、模拟器、截图或可渲染预览检查；涉及 Web 镜像时同步截图/浏览器预览。发现错位、比例失真、文字挤压、遮挡或红框类明显问题时，必须先修正再提交/发布。

## 圆角系统

| 场景 | 圆角 |
|---|---:|
| 小图块 / 项目封面 | `6dp` |
| 普通按钮 | `20dp` 到 `24dp` |
| 搜索框 | `28dp` |
| 筛选胶囊 | `24dp` |
| 卡片 | `16dp` 到 `18dp` |
| 大卡片 / 公告容器 | `20dp` |
| 圆形悬浮按钮 | `999dp` |
| 白色浮层菜单 | `14dp` 到 `16dp` |
| AI 会话胶囊 | `999dp` |

圆角规则：

- 卡片圆角要明显，但不要做拟物阴影。
- 胶囊控件必须左右完全圆润，适合搜索、筛选、主操作和 AI 会话入口。
- 项目封面占位图保持小圆角，不能变成胶囊或大圆角卡片。

## 导航

### 一级项目页顶部

- 顶部使用两个大标题 Tab：“我的项目”“项目广场”。
- 选中态：标题下方短横线，宽 `32dp` 到 `40dp`，高 `2dp`，圆角 `999dp`。
- 未选中态：同字号同字重，同样使用 `color.text.primary`；只通过下划线表达当前选中。
- 右上角新增按钮为暗色圆形，直径 `56dp` 到 `60dp`，背景 `color.surface.float`，描边 `color.border.float`，图标使用 `color.icon.add.top`。
- 顶部标题和加号必须在视觉上同一行，不使用传统 AppBar 背板。

### 详情页顶部

- 详情页使用原生导航感：左侧返回，居中项目名，右侧成员/管理图标。
- 顶部返回图标使用 `color.text.detail.title`，右侧成员图标使用图 3 采样色 `color.icon.member`。
- 标题居中，复用字体表中的页面标题层级，不额外新增详情页字号。

### 底部导航

- 主页显示“长胶囊 + 独立圆钮”的悬浮导航：长胶囊内依次为好友、项目、我的、主菜单，右侧独立圆钮用于新建会话。
- 长胶囊和独立圆钮可视高度均为 `56dp`，左右页面边距 `20dp`，两者间距 `12dp`；外层顶部和底部各保留 `8dp`。
- 好友、项目、我的使用用户确认的明暗两套 PNG 素材；主菜单使用默认与抽屉展开两套 PNG 素材。所有入口不显示文字标签，触控区域仍不得小于 `48dp x 48dp`，并保留 `contentDescription`。
- 当前页使用浅灰选中胶囊和深色图标；其余主页面入口使用浅色图标。主菜单只负责开关项目抽屉：展开时仅三横杠切换为 `color.icon.menu.open` 蓝色素材，关闭时恢复默认素材，好友、项目、我的当前选中态全程不变。
- 主菜单不显示选中胶囊，也不占用主页面选中态。主页面选中胶囊尺寸为 `58dp x 46dp`；最左好友入口距离长胶囊左、上、下边均为 `5dp`，并且不得随屏幕宽度变化。
- 长胶囊、选中胶囊和独立圆钮均使用用户提供的原始背景素材，不额外叠加阴影、渐变或高亮线。
- 底部可视组件的底边必须与聊天输入胶囊共用 `8dp` 基准间距；系统导航栏 / 手势安全区在该基准之外叠加，页面切换不得产生纵向跳变。

## 搜索与筛选

### 搜索框

- 搜索框高度 `56dp`，左右边距 `16dp` 到 `20dp`。
- 背景 `color.surface.search`，圆角 `28dp`。
- 搜索图标 `24dp`，左内边距 `20dp`。
- 占位文字使用字体表中的主文本层级，颜色 `color.text.placeholder`。
- 搜索框下方到筛选条距离 `16dp` 到 `20dp`。

### 筛选胶囊

- 筛选项横向排列，间距 `10dp` 到 `16dp`。
- 选中项为深灰胶囊，背景 `color.surface.segment.selected`，内边距左右 `20dp`，高度 `48dp`。
- 未选中项无背景，文字使用 `color.text.primary`。
- 项目广场筛选顺序建议：全部、可安装、无审批、已加入、最热门。
- 我的项目分段建议：独立、联合。

## 列表项

我的项目列表采用无卡片列表，而不是每行一张深灰卡。

- 列表行背景透明，直接落在黑色页面上。
- 项目封面为白色或真实封面图，尺寸 `56dp` 到 `64dp`，圆角 `6dp`。
- 文本区从封面右侧开始，标题在第一行，简介在第二行，元信息在第三行。
- 右侧箭头使用线性 chevron，颜色 `color.text.placeholder`。
- 行高建议 `112dp` 到 `128dp`；行间距通过底部 padding 控制，不画分割线。
- 长标题单行省略，简介可单行省略，元信息保持一行。
- 元信息格式示例：`创建者：叶云    成员：1`。

## 项目广场卡片

项目广场使用大圆角深灰卡片，强调项目可加入、可安装、是否审批。

### 卡片结构

- 卡片左右边距 `16dp`，圆角 `18dp`。
- 卡片头部高度 `52dp` 到 `56dp`，背景 `color.surface.header`。
- 卡片主体背景 `color.surface.card`。
- 标题放在头部左侧，复用字体表中的主文本或个人卡片标题层级。
- 状态放在头部右侧，使用文字加小圆点。
- 主体左侧为项目封面，右侧/下方为创建者、成员、时间、简介和按钮。
- 简介和上方信息之间可使用短分割线，长度约为内容区 `65%`，不要贯穿整卡。

### 状态表达

- “无需审批”“可安装”使用绿色小圆点。
- “需审批”使用红色小圆点。
- 状态文字颜色使用 `color.text.primary`，圆点承担状态颜色。
- 小圆点直径 `6dp` 到 `7dp`，与文字基线垂直居中。

### 卡片按钮

- 主操作按钮为白底黑字胶囊，最小高度 `44dp`。
- 卡片内可并列两个按钮：“进入空间”“下载APK”。
- 两按钮之间间距 `12dp` 到 `16dp`，宽度保持一致或按内容稍微自适应。
- 按钮文字复用字体表中的主文本层级，字重可加粗。
- 不使用绿色按钮作为“进入空间”或“下载APK”的主样式。

## 项目空间详情页

项目空间详情页是“黑色内容画布 + 顶部公告 + 悬浮操作”的结构。

- 页面顶部背景使用 `color.bg.chrome`，内容画布使用 `color.bg.canvas`。
- 顶部公告容器背景 `color.surface.header`，圆角 `20dp`。
- 公告标题“公告”和正文复用字体表中的主文本层级，颜色 `color.text.placeholder`。
- 公告容器右侧可放三横线菜单按钮，图标使用 `color.icon.primary`，触控区不小于 `48dp`。
- 公告下方内容区使用 `color.bg.canvas`，顶部圆角可与公告形成连续容器感。
- 空状态文案居中偏上，颜色 `color.text.placeholder`，复用字体表中的主文本层级。
- 空状态下方的加号按钮为暗色圆形，直径 `56dp`，背景 `color.surface.float`。
- 右下角 AI 会话入口为白色大胶囊，高度 `56dp` 到 `60dp`，右边距 `20dp`，底部避开安全区。

## 弹出菜单

- 弹出菜单使用白色背景 `color.surface.inverse`，圆角 `14dp` 到 `16dp`。
- 菜单文字使用 `color.text.menu`，复用字体表中的主文本层级。
- 项目空间菜单项示例：项目文档、下载APK。
- 菜单靠触发图标右侧或下方浮出，使用小三角指向触发点。
- 菜单不加深色边框；可使用轻微透明遮罩或无遮罩。
- 每个菜单项高度不小于 `36dp`，左右内边距 `18dp`。

## 悬浮操作

### 新建圆形按钮

- 顶部新增项目按钮：圆形，直径 `56dp` 到 `60dp`。
- 内容区新增帖子按钮：圆形，直径 `56dp`。
- 背景 `color.surface.float`，顶部加号使用 `color.icon.add.top`，内容区加号使用 `color.icon.primary`。
- 可带 `1dp` 弱描边；不要使用投影或彩色底。

### AI 会话按钮

- 固定右下角，白底黑字。
- 高度 `56dp` 到 `60dp`，圆角 `999dp`。
- 左侧使用编辑/会话线性图标，右侧文字 `AI 会话`。
- 文字复用字体表中的主文本层级，字重 `400`。
- 图标和文字间距 `14dp`。

## 图标规范

- 图标风格统一为线性轮廓，圆角端点，描边约 `2dp`。
- 常用图标：返回、加号、搜索、三横线、成员、项目、好友、我的、编辑。
- 不使用彩色图标表达普通导航状态。
- 图标颜色只能从 `color.text.primary`、`color.text.secondary`、`color.text.nav`、`color.icon.member`、`color.icon.primary`、`color.icon.add.top` 中选择；底部主菜单抽屉展开时可例外使用 `color.icon.menu.open`。
- 图标按钮必须有足够触控区，视觉图标可小，点击范围不能小。

## 图片与占位

- 项目封面占位可用白色块，圆角 `6dp`，尺寸 `56dp` 到 `64dp`。
- 有真实封面时使用等比裁剪填充，不拉伸。
- 封面不要加厚边框；在深色背景上白色占位已经足够醒目。
- 图片加载失败时显示白色或浅灰占位，不显示破图标。

## 交互状态

- 按下态：保持原始填充 token，不新增 HEX；全 APP 禁用系统无边界 Ripple，保留控件自身的轻微缩放、透明度、状态切换和业务动画，有边界反馈仍按具体组件需要使用。
- 禁用态：沿用原始 token，整体 alpha 降到 `40%`；不得引入新的禁用灰色。
- 加载态：优先在按钮内显示小型进度或禁用按钮，不改变布局高度。
- 选中态：导航用下划线或深灰胶囊，底部主页面导航用亮度；`color.icon.menu.open` 只表达项目抽屉已展开，不代表主页面被选中。
- 错误态：只在状态点、文案或局部提示使用红色。

## 可访问性

- 主要文字与背景对比度必须满足可读性，避免 `color.text.quiet` 承载长正文。
- 所有点击目标不小于 `48dp x 48dp`。
- 纯图标按钮必须有 `contentDescription`。
- 状态不能只靠颜色表达，必须同时有文字，例如“需审批”“可安装”。
- 重要按钮文案使用明确动词，不只写“确定”或“进入”。

## Android 与 Web 对齐

修改 APK UI、主题、颜色、按钮、卡片、底部导航或项目空间结构时，必须同步检查 Web 镜像页面 `server/src/assets/web_page.html`。颜色变量和组件结构应与本文保持一致。

Android 资源建议：

- 颜色集中放在 `android/app/src/main/res/values/colors.xml`。
- 页面主题集中放在 `android/app/src/main/res/values/themes.xml`。
- 新增颜色必须先补充本文 token，再补资源文件。
- Kotlin 中临时写死颜色只允许短期过渡；新增组件优先引用资源或统一常量。

Web 变量建议：

- `--bg-mine` 对应 `color.bg.mine`。
- `--bg-plaza` 对应 `color.bg.plaza`。
- `--bg-chrome` 对应 `color.bg.chrome`。
- `--bg-canvas` 对应 `color.bg.canvas`。
- `--ink-list-preview` 对应 `color.text.list.preview`。
- `--panel` 对应 `color.surface.card`。
- `--panel-header` 对应 `color.surface.header`。
- `--segment-selected` 对应 `color.surface.segment.selected`。
- `--search-bg` 对应 `color.surface.search`。
- `--brand` 对应白色主按钮背景，而不是绿色。
- `--brand-ink` 对应黑色按钮文字。
- `--success`、`--danger` 只用于状态点和状态文本。

## 禁止项

- 禁止把绿色作为大面积主按钮背景，除非是明确的成功状态。
- 禁止引入蓝色、紫色、橙色作为新的主辅助色；`color.icon.menu.open` 仅限底部主菜单抽屉展开反馈。
- 禁止使用渐变背景、光斑、装饰球、拟物投影。
- 禁止用卡片包裹所有列表项；我的项目列表应保持透明行。
- 禁止在深色卡片内再套一层同样视觉重量的卡片。
- 禁止随意新增黑灰色阶，导致页面出现脏灰、层级混乱。
- 中文正文必须复用字体表中的层级；`font.meta.small` 只用于时间、版本号、辅助提示。

## 页面套用摘要

| 页面 | 主结构 | 关键组件 |
|---|---|---|
| 我的项目 | 黑底无卡片列表 | 大 Tab、分段胶囊、项目封面、右箭头、底部导航 |
| 项目广场 | 黑底大卡片流 | 搜索框、筛选胶囊、项目卡片、状态点、白色操作按钮 |
| 项目空间 | 顶部导航 + 公告 + 深黑内容画布 | 公告容器、菜单浮层、空状态加号、右下 AI 会话按钮 |
| 个人/我的 | 深色分组列表 | 底部导航、深灰行、白色或中性操作入口 |
| 聊天/好友 | 黑底消息/列表 | 线性图标、白色主动作、深灰输入胶囊 |

## 更新流程

- 修改 UI 前先读本文件。
- 若设计需要新增颜色、字号、圆角或组件形态，先更新本文件。
- 若只是在现有规范内实现页面，不要扩展 token。
- APK 与 Web 同源功能必须保持视觉一致。
- 规范变更应在提交说明中明确写出“更新 APP UI 设计规范”。
