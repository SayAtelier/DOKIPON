import json, pathlib, re, shutil
src=pathlib.Path('/mnt/data/gemarc-5d-flutter');out=pathlib.Path('/mnt/data/GemArc-5D-Flutter-Design-Atlas')
load=lambda n:json.loads((src/'assets/models'/n).read_text(encoding='utf8'))
arch=load('architecture.json'); flows=load('causal_flows.json'); cases=load('causal_cases.json'); wf=load('workflow.json')
# Design model: planned Flutter system, never confuse it with observed Feliture internal architecture.
planned=[
('app','Flutter Commerce App','System','Flutter 共享体验','Designed','跨端共享 Material/Cupertino 页面层、业务状态及领域逻辑'),
('shell','App Shell / Router','Component','体验壳','Designed','自适应布局，深链跳转及权限路由'),
('design','Design Tokens / i18n','Component','设计系统','Designed','zh-CN、ja-JP、en-US；语义颜色、字号、平台间距和无障碍'),
('home-ui','Home / Campaign Screen','Page','体验壳','Designed','首页、营销区、导航和推荐模块'),
('catalog-ui','Catalog & Search','Page','商品体验','Designed','筛选、排序、详情和商品变体'),
('fav-ui','Wishlist','Page','商品体验','Designed','收藏本地临时状态与登录用户同步'),
('cart-ui','Cart & Checkout','Page','交易体验','Designed','购物车、结算、请求幂等、失败恢复'),
('orders-ui','Order Tracking','Page','交易体验','Designed','订单状态跟踪、售后与客服入口'),
('session','Session / Auth Controller','Controller','应用领域','Designed','匿名态与用户态切换，最小信息原则'),
('catalog-repo','Catalog Repository','Repository','应用领域','Designed','缓存策略、分页、可观察的数据源'),
('checkout-state','Checkout State Machine','Controller','应用领域','Designed','Guard、不可重复提交、超时恢复'),
('storage','Local Secure Cache','Adapter','本地存储','Designed','非敏感离线缓存；敏感值使用平台安全存储策略'),
('transport','Commerce API Client','Adapter','平台基础设施','Designed','网络异常、重试/取消、错误码映射、鉴权处理'),
('mobile','Android / iOS Shell','Platform','发布通道','Designed','Flutter 原生目标、移动适配和通知授权'),
('web','Flutter Web Shell','Platform','发布通道','Designed','路由、缓存策略和响应式布局'),
('line','LIFF Web Adapter','Platform','发布通道','Designed','LINE MINI App 运行在 WebView；官方 SDK 桥接与审核'),
('identity-backend','Identity / BFF API','Service','后端合同','Hypothetical','尚无业务源码；此节点是建议接口契约而非实际服务'),
('commerce-backend','Commerce Domain API','Service','后端合同','Hypothetical','商品、购物车、订单 API 均为待业务确认契约'),
('payment-boundary','Payment Gateway Boundary','External','外部边界','Hypothetical','不得在客户端持有支付密钥，服务端结算'),
]
plannedNodes=[{'id':'f-'+i,'name':name,'kind':kind,'zone':zone,'verification':v,'description':desc,'evidenceIds':[],'layer':'flutter-design'} for i,name,kind,zone,v,desc in planned]
conn=[('app','shell','contains'),('app','design','uses'),('shell','home-ui','routes'),('shell','catalog-ui','routes'),('shell','fav-ui','routes'),('shell','cart-ui','routes'),('shell','orders-ui','routes'),('catalog-ui','catalog-repo','reads'),('home-ui','catalog-repo','reads'),('cart-ui','checkout-state','commands'),('checkout-state','transport','calls'),('catalog-repo','transport','calls'),('catalog-repo','storage','caches'),('app','session','uses'),('session','storage','persists'),('transport','identity-backend','proposed-api'),('transport','commerce-backend','proposed-api'),('commerce-backend','payment-boundary','proposed-api'),('mobile','app','hosts'),('web','app','hosts'),('line','web','hosts'),('line','session','bridges'),('orders-ui','transport','reads'),('fav-ui','storage','caches'),('cart-ui','storage','drafts'),('session','transport','authenticates')]
plannedEdges=[{'id':'fe-'+str(j).zfill(2),'sourceId':'f-'+s,'targetId':'f-'+t,'type':type,'verification':'Hypothetical' if s in ('identity-backend','commerce-backend') or t in ('identity-backend','commerce-backend','payment-boundary') else 'Designed','directed':True,'evidenceIds':[],'layer':'flutter-design'} for j,(s,t,type) in enumerate(conn,1)]
features=[
{'id':'FR-APP-01','name':'跨端一致体验','priority':'Must','scope':'Flutter Web/Android/iOS + LINE WebView','description':'共享页面状态、设计 Token、业务语义；按平台提供独立导航和授权适配。','accept':'同一商品和订单状态在支持的端上保持语义一致；明确各端未支持的 API。','design':'BD-APP-01','detail':'DD-APP-01','case':'CASE-RELEASE-001'},
{'id':'FR-APP-02','name':'公开商品浏览','priority':'Must','scope':'首页 / 列表 / 详情','description':'首页、商品分类、商品详情、尺码与帮助等以现有公开站点为业务线索重新设计。','accept':'商品卡片进入详情；筛选状态可恢复；空结果与网络故障有独立设计。','design':'BD-CATALOG-01','detail':'DD-CATALOG-01','case':'CASE-CATALOG-001'},
{'id':'FR-APP-03','name':'收藏与购物车','priority':'Must','scope':'收藏 / 购物车','description':'收藏与购物车作为 App 设计能力，服务端可用性与现站点实际能力尚待确认。','accept':'匿名与已登录状态、数量越界、重试冲突都有明确状态转换。','design':'BD-CHECKOUT-01','detail':'DD-CHECKOUT-01','case':'CASE-CHECKOUT-002'},
{'id':'FR-APP-04','name':'结算与订单','priority':'Must','scope':'结算 / 订单 / 售后','description':'库存、支付授权、订单写入采用单次提交与幂等设计，并定义超时后的查账补偿。','accept':'异常支付不重复扣款；终态包含成功、失败、待核验／恢复。','design':'BD-CHECKOUT-01','detail':'DD-CHECKOUT-01','case':'CASE-CHECKOUT-005'},
{'id':'FR-APP-05','name':'LINE MINI App','priority':'Should','scope':'LIFF / LINE WebView','description':'Flutter Web 作为 LINE WebView 的页面资源；LIFF SDK 仅在 Web Adapter 内调用。','accept':'内嵌与外部浏览器均有降级策略；登录失败不暴露令牌；发布需通过平台审核。','design':'BD-LINE-01','detail':'DD-LINE-01','case':'CASE-LINE-003'},
{'id':'FR-APP-06','name':'3 语言本地化','priority':'Must','scope':'中文 / 日语 / 英语','description':'页面文案、错误信息、长文本、安全提示采用语言字典；日语文案降低认知负担。','accept':'可切换 zh-CN / ja-JP / en-US；文字不超框、货币与日期使用本地格式。','design':'BD-UI-01','detail':'DD-UI-01','case':'CASE-RELEASE-008'},
{'id':'FR-5D-00','name':'0D 证据与事实分级','priority':'Must','scope':'证据台账','description':'Observed、Implemented、Designed、Inferred、Hypothetical、Unknown 显式标记。','accept':'无证据的后台推断不能标记为 Observed。','design':'BD-EVIDENCE-01','detail':'DD-EVIDENCE-01','case':'CASE-EVIDENCE-001'},
{'id':'FR-5D-01','name':'1D/2D Graph + UML/ER','priority':'Must','scope':'架构查阅','description':'节点和边独立拾取，查阅证据、相邻节点及 UML/ER 语义。','accept':'每条 Edge 独立可点击，关系类型与端点 ID 正确。','design':'BD-GRAPH-01','detail':'DD-GRAPH-01','case':'CASE-GRAPH-001'},
{'id':'FR-5D-03','name':'3D 楼宇楼层房间','priority':'Must','scope':'空间架构','description':'Building/Floor/Room 映射系统、层与子域，不冒充真实物理设施。','accept':'楼层可展开/收起，房间可聚焦，动态退避保留关系。','design':'BD-SPATIAL-01','detail':'DD-SPATIAL-01','case':'CASE-SPATIAL-001'},
{'id':'FR-5D-04','name':'4D 事件时间线','priority':'Must','scope':'事件回放','description':'时间序列包括事件、状态、节点 ID 与动画类别，支持暂停与逐帧。','accept':'相同输入得到相同轨迹；可逐帧看到状态差分。','design':'BD-CAUSAL-01','detail':'DD-CAUSAL-01','case':'CASE-CAUSAL-001'},
{'id':'FR-5D-05','name':'5D 因果分歧与反事实','priority':'Must','scope':'正常/异常 Case','description':'守卫条件、注入变量、正常基线、异常路径、补偿、组合故障和边界条件。','accept':'Case 改变输入会重新计算轨迹，不能只换动画颜色。','design':'BD-CAUSAL-01','detail':'DD-CAUSAL-01','case':'CASE-CAUSAL-009'},
{'id':'FR-WF-01','name':'AI 原生无限画布','priority':'Must','scope':'节点/端口/边','description':'画布坐标与工作流执行语义分离；结构验证与导出契约保真。','accept':'平移缩放不改变执行图；端口与边端点可逐一核验。','design':'BD-WORKFLOW-EDIT-01','detail':'DD-WORKFLOW-EDIT-01','case':'CASE-WORKFLOW-EDIT-001'},
{'id':'FR-QA-01','name':'IPA 风格双向追踪','priority':'Must','scope':'需求→设计→Case','description':'需求定义、基本设计、详细设计、测试、异常恢复与证据可交叉定位。','accept':'每项需求具备设计 ID、测试 Case、预期结果及验证等级。','design':'BD-EVIDENCE-01','detail':'DD-EVIDENCE-01','case':'CASE-EVIDENCE-002'}
]
platforms=[
{'name':'Flutter Web','short':'WEB','channel':'浏览器和 PC','source':'Dart 共享业务状态 + Flutter Web 页面','integration':'URL Router / HTTP API / 浏览器持久化策略','constraint':'首次加载体积、SEO、浏览器安全限制','gate':'Chrome/Edge/Safari 浏览器与屏幕阅读器测试'},
{'name':'Android App','short':'AND','channel':'移动原生','source':'共享 Domain / UI 与 Android Plugin 适配层','integration':'App Links / Secure Storage / 通知授权','constraint':'权限声明、应用商店政策、签名密钥','gate':'多分辨率、系统退后台、弱网、通知权限'},
{'name':'iOS App','short':'IOS','channel':'移动原生','source':'共享 Domain / UI 与 iOS Platform Channel','integration':'Universal Links / Keychain / 推送授权','constraint':'App Review、隐私清单、系统返回语义','gate':'真机、VoiceOver、弱网与审核场景'},
{'name':'LINE MINI App','short':'LINE','channel':'LINE App 内置 WebView','source':'Flutter Web 构建物 + LIFF JavaScript Bridge','integration':'liff.init / 外部浏览器降级 / Server-side auth','constraint':'LINE 平台审核、商品与内容政策、WebView 能力','gate':'LINE 内/外测试、登录取消、授权恢复、审核通过'}
]
data={'meta':{'version':'5.2','name':'Feliture Flutter 0D–5D 设计图谱','source':'https://feliture.shop/','assessed':'2026-10-17','publicEvidence':len(arch['evidence']),'note':'Public website observation is not source access. Flutter target and its system design are Design/Hypothetical.'},'observedGraph':arch,'designGraph':{'nodes':plannedNodes,'edges':plannedEdges},'features':features,'platforms':platforms,'flows':flows['flows'],'cases':cases['cases'],'workflow':wf,'docs':[]}
for p in sorted((src/'docs').rglob('*.md')):
 rel=p.relative_to(src/'docs').as_posix(); txt=p.read_text(encoding='utf8')
 data['docs'].append({'id':rel,'title':p.stem.replace('_',' '),'body':txt,'size':len(txt),'group':rel.split('/')[0]})
 # keep independent markdown sources too
 target=out/'design-docs'/rel;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,target)
for n in ('architecture.json','causal_cases.json','causal_flows.json','workflow.json'):
 shutil.copy2(src/'assets/models'/n,out/'reference-models'/n)
(out/'data.js').write_text('/* Inlined for offline file:// opening; immutable design source. */\nwindow.GEMARC_DESIGN_DATA = '+json.dumps(data,ensure_ascii=False,separators=(',',':'))+';\n',encoding='utf8')
print({'documents':len(data['docs']),'cases':len(data['cases']),'flows':len(data['flows']),'source_nodes':len(arch['nodes']),'design_nodes':len(plannedNodes),'planned_edges':len(plannedEdges),'requirements':len(features),'data_bytes':(out/'data.js').stat().st_size})
