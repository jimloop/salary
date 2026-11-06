const pages = {
  subjects: ["主体列表", "选择并解锁一个主体，或创建新的独立工作空间。"],
  overview: ["工作台", "聚焦当前主体的导入、规则和计算状态。"],
  employees: ["员工与薪酬方案", "维护当前主体的人员、部门、成本中心和有效薪酬方案。"],
  items: ["工资项目", "先定义收入、扣款和成本项目，再逐项配置计算规则。"],
  import: ["数据导入", "先核对主体与字段，再决定是否写入本主体的输入批次。"],
  rules: ["规则配置", "每条规则都有适用范围、有效期、来源和独立版本。"],
  payroll: ["月薪计算", "从输入完整度到逐项解释，清晰完成当期试算。"],
  bonus: ["年终奖计算", "查看资格、预算与两种计税方式的模拟结果。"],
  employee: ["员工计算明细", "追溯原始值、规则版本、取整步骤与期间差异。"],
  history: ["版本与异常", "保留每次计算快照，定位变化与阻断问题。"],
  export: ["导出与备份", "导出当前主体计算结果，管理单主体离线备份。"]
};
const $ = s => document.querySelector(s);
let current = "subjects";
const prototypeData = {
  subjects: [{id:"E-A",name:"云杉科技",tax:"3482",region:"上海",opened:"2026-11-04 15:42"},{id:"E-B",name:"海棠咨询",tax:"9107",region:"杭州",opened:"2026-10-30 09:18"}],
  active:"E-A",
  employees:[{id:"A-1001",name:"林知远",department:"产品部",cost:"CC-01",type:"正式员工",hire:"2024-03-01",salary:"10000.00"},{id:"A-1002",name:"陈亦凡",department:"研发部",cost:"CC-02",type:"正式员工",hire:"2025-07-01",salary:"12000.00"}],
  items:[],
  rules:[]
};
let selectedItem = "fixed_salary";
const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;"}[c]));
const subjectRecords = new Map([[prototypeData.active, {employees:prototypeData.employees, items:prototypeData.items, rules:prototypeData.rules}]]);
function renderSwitcher() {
  const active = prototypeData.subjects.find(subject => subject.id === prototypeData.active);
  $("#prototypeEntityName").textContent = active?.name || "请选择主体";
  $("#prototypeEntityTax").textContent = active ? `税号末位 · ${active.tax}` : "新建或选择工作空间";
  $("#entityAvatar").textContent = active?.name.slice(0, 1) || "+";
  document.querySelector('[data-page="import"] em').textContent = "";
  $("#entityMenu").innerHTML = `<div class="entity-menu-title">切换主体</div>${prototypeData.subjects.map(subject => `<button class="entity-option ${subject.id === active?.id ? "selected" : ""}" type="button" role="menuitemradio" aria-checked="${subject.id === active?.id}" data-action="choose-subject-${esc(subject.id)}"><span class="entity-option-avatar">${esc(subject.name.slice(0,1))}</span><span class="entity-option-copy"><strong>${esc(subject.name)}</strong><small>主体标识 ${esc(subject.id)} · 税号末位 ${esc(subject.tax)}</small></span><span class="entity-option-check" aria-hidden="true">${subject.id === active?.id ? "✓" : ""}</span></button>`).join("")}${prototypeData.subjects.length ? "" : '<div class="entity-menu-empty">暂无主体工作空间</div>'}<button class="entity-menu-footer" type="button" data-action="view-all-subjects">查看全部主体 <span aria-hidden="true">→</span></button>`;
}
function setSwitcherOpen(open) {
  $("#entityMenu").hidden = !open;
  $("#entityToggle").setAttribute("aria-expanded", String(open));
  if (open) $("#entityMenu .entity-option")?.focus();
}
function switchSubject(id) {
  if (!prototypeData.subjects.some(subject => subject.id === id)) return;
  if (id !== prototypeData.active) {
    subjectRecords.set(prototypeData.active, {employees:prototypeData.employees, items:prototypeData.items, rules:prototypeData.rules});
    const records = subjectRecords.get(id) || {employees:[], items:[], rules:[]};
    prototypeData.active = id;
    prototypeData.employees = records.employees;
    prototypeData.items = records.items;
    prototypeData.rules = records.rules;
    selectedItem = records.items[0]?.code || "";
    employeeQuery = "";
    employeeType = "全部类型";
    itemType = "全部项目";
    for (const state of pagerState.values()) state.page = 1;
  }
  setSwitcherOpen(false);
  render("subjects");
}
const btn = (name, action, primary = false) => `<button class="btn ${primary ? "primary" : ""}" data-action="${action}">${name}</button>`;
const badge = (name, tone = "") => `<span class="badge ${tone}">${name}</span>`;
const table = (heads, rows) => `<div class="table-wrap"><table class="table"><thead><tr>${heads.map((h, i) => `<th ${i===0?'class="sticky"':''}>${h}</th>`).join("")}</tr></thead><tbody>${rows.map(row => `<tr>${row.map((v,i) => `<td ${i===0?'class="sticky"':''}>${v}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
const pagerState = new Map();
function paginate(key, records, alwaysShow = false) {
  const state = pagerState.get(key) || {page:1, size:20};
  const pages = Math.max(1, Math.ceil(records.length / state.size));
  state.page = Math.min(Math.max(1, state.page), pages);
  pagerState.set(key, state);
  const visible = records.slice((state.page - 1) * state.size, state.page * state.size);
  if (!alwaysShow && pages === 1) return {visible, footer:""};
  const numbers = [...new Set([1, state.page - 1, state.page, state.page + 1, pages].filter(number => number >= 1 && number <= pages))].sort((a,b) => a-b);
  const pageButtons = numbers.map((number, index) => `${index && number > numbers[index-1] + 1 ? '<span class="pager-ellipsis" aria-hidden="true">…</span>' : ""}<button type="button" class="pager-page ${number === state.page ? "active" : ""}" data-pager-key="${key}" data-pager-page="${number}" aria-label="第 ${number} 页" ${number === state.page ? 'aria-current="page"' : ""}>${number}</button>`).join("");
  const pagerLabel = {employees:"员工",items:"工资项目",subjects:"主体","export-audit":"操作日志"}[key] || "记录";
  return {visible, footer:`<div class="pager" role="navigation" aria-label="${pagerLabel}分页"><span class="pager-total">共 ${records.length} 条</span><label>每页 <select data-pager-size="${key}" aria-label="每页显示条数">${[20,50,100].map(size => `<option value="${size}" ${size === state.size ? "selected" : ""}>${size}</option>`).join("")}</select> 条</label><div class="pager-buttons"><button type="button" data-pager-key="${key}" data-pager-page="${state.page-1}" aria-label="上一页" ${state.page === 1 ? "disabled" : ""}>‹</button>${pageButtons}<button type="button" data-pager-key="${key}" data-pager-page="${state.page+1}" aria-label="下一页" ${state.page === pages ? "disabled" : ""}>›</button></div><span class="pager-count">第 ${state.page} / ${pages} 页</span></div>`};
}
let employeeQuery = "", employeeType = "全部类型", itemType = "全部项目";
const panels = {
  bonus: () => `<div class="form-row"><label class="field">奖金年度<select><option>2026</option></select></label><label class="field wide">方案<select><option>年度绩效奖金 · 方案 v2</option></select></label><label class="field">计划发放月<input type="month" value="2027-01"></label></div><div class="cards"><div class="card"><div class="metric-label">预算池</div><div class="metric-value">¥180万</div><div class="metric-sub">由本主体方案输入</div></div><div class="card"><div class="metric-label">税前分配</div><div class="metric-value">¥186.2万</div><div class="metric-sub">含调整项</div></div><div class="card"><div class="metric-label">预算差额</div><div class="metric-value" style="color:#ae6b20">+¥6.2万</div><div class="metric-sub">超预算警告</div></div><div class="card"><div class="metric-label">符合资格</div><div class="metric-value">392</div><div class="metric-sub">36 人不符合资格</div></div></div><div class="two-cols"><div class="card"><div class="section-head"><h2>个人奖金与资格</h2><small>预算超额仅提示，不产生审批流程</small></div>${table(["工号", "姓名", "资格", "基础奖金", "绩效系数", "在职系数", "税前奖金"], [["A-1001","林知远",badge("符合"),"¥20,000","1.2","1.0","¥24,000"],["A-1002","陈亦凡",badge("符合"),"¥18,000","1.0","1.0","¥18,000"],["A-1003","周书语",badge("待确认", "warn"),"¥14,000","0.9","0.5","¥6,300"]])}</div><div class="card"><div class="section-head"><h2>计税方式模拟</h2>${badge("需确认外部事实", "warn")}</div><div class="list-row"><div><strong>全年一次性奖金单独计税</strong><small>需确认本纳税年度未在其他主体或雇主使用</small></div></div><div class="list-row"><div><strong>并入综合所得</strong><small>依赖本主体历史实际发放与扣除基线</small></div></div><div class="issue"><span>!</span><div><strong>两种结果均为模拟</strong>应用不自动选择更优方案；未知历史税额不会当作零。</div></div></div></div>`,
  history: () => `<div class="two-cols"><div class="card"><div class="section-head"><h2>计算版本</h2><small>每次重算生成新版本</small></div>${table(["版本", "所属期", "计划发放", "规则集", "人数", "时间", "操作"], [["v3","2026-10","2026-11","规则集 v3","428","11-03 14:42",btn("对比 v2", "demo-compare")],["v2","2026-10","2026-11","规则集 v2","428","11-03 11:25",btn("查看快照", "demo-snapshot")],["v1","2026-10","2026-11","规则集 v2","427","11-02 16:11",btn("查看快照", "demo-snapshot")]])}<div class="label">v3 与 v2 · 主要变化</div><div class="list-row"><div><strong>林知远 · 津贴 +¥800.00</strong><small>来源：绩效补录；规则版本未变</small></div></div><div class="list-row"><div><strong>周书语 · 缺勤扣款 −¥454.55</strong><small>来源：考勤批次修正；规则 v3</small></div></div></div><div class="card"><div class="section-head"><h2>异常清单</h2>${badge("2 阻断 · 4 警告", "warn")}</div><div class="issue danger"><span>!</span><div><strong>A-1002 · 历史实际税额缺失</strong>历史实际数据批次 #99 无该员工。导入完整收入、扣除与已预扣税额后重算。</div></div><div class="issue danger"><span>!</span><div><strong>A-1028 · 预估实发为负</strong>检查补扣款原始文件第 17 行与对应规则。</div></div><div class="issue"><span>!</span><div><strong>A-1003 · 金额环比增加 32%</strong>对比考勤修正和绩效输入。</div></div></div></div>`,
  export: () => `<div class="two-cols"><div class="card"><div class="section-head"><h2>计算结果导出</h2>${badge("仅当前主体", "blue")}</div><div class="form-row"><label class="field">计算版本<select><option>2026-10 · 试算 v3</option></select></label><label class="field">文件内容<select><option>员工逐项明细</option><option>项目汇总</option><option>异常清单</option><option>计算说明</option></select></label></div><div class="list-row"><div><strong>导出文件将包含</strong><small>主体、所属期、计划发放月、版本、生成时间、仅供薪酬计算使用</small></div></div><div class="issue danger"><span>!</span><div><strong>目前不能导出完整结果</strong>2 位员工税额基线缺失，1 位员工预估实发为负。先修正阻断项，再验证工作空间密码。</div></div>${btn("查看阻断异常", "goto-history")}</div><div class="card"><div class="section-head"><h2>单主体离线备份</h2><small>不包含其他主体</small></div><p class="helper">备份文件包含主体 ID、加密数据和完整性校验。恢复前校验主体与文件完整性；不同主体或损坏文件将拒绝写入。</p><div class="list-row"><div><strong>上次备份</strong><small>2026-10-31 18:22 · 外部介质</small></div><span class="tail">${badge("已验证")}</span></div><div class="actions" style="margin-top:14px">${btn("创建加密备份", "demo-backup", true)}${btn("从备份恢复", "demo-restore")}</div><div class="label">操作日志</div><div class="list-row"><div><strong>11-03 14:42 · 计算试算 v3</strong><small>本主体薪酬计算员 · 保留输入与规则快照</small></div></div></div></div>`
};
panels.subjects = () => {
  const {visible, footer} = paginate("subjects", prototypeData.subjects);
  return `<div class="section-head"><h2>本机工作空间</h2></div>${prototypeData.subjects.length ? `<div class="subject-grid">${visible.map(subject => `<div class="card subject-card"><div class="subject-icon">${esc(subject.name.slice(0,1))}</div><div><strong>${esc(subject.name)}</strong><small>主体标识 ${esc(subject.id)} · 税号末位 ${esc(subject.tax)} · ${esc(subject.region)}</small></div><div class="subject-bottom"><span>上次打开 ${esc(subject.opened)}</span><div class="subject-actions">${btn(prototypeData.active===subject.id?"当前主体":"切换到此主体", `unlock-${esc(subject.id)}`, prototypeData.active!==subject.id)}<button class="btn delete-subject" type="button" aria-label="删除${esc(subject.name)}及其全部数据">删除</button></div></div></div>`).join("")}</div>${footer}` : `<div class="empty"><strong>还没有主体工作空间</strong>新建主体后即可录入员工、工资项目与计算规则。<div style="margin-top:14px">${btn("新建主体", "new-subject", true)}</div></div>`}<p class="helper">切换主体后，员工、工资项目和规则仅显示所选主体的数据。</p>`;
};
panels.employees = () => {
  const filtered = prototypeData.employees.filter(employee => (!employeeQuery || [employee.id, employee.name, employee.department].some(value => value.toLowerCase().includes(employeeQuery.toLowerCase()))) && (employeeType === "全部类型" || employee.type === employeeType));
  const {visible, footer} = paginate("employees", filtered, true);
  return `<div class="form-row"><label class="field wide">搜索员工<input type="search" id="prototypeEmployeeSearch" value="${esc(employeeQuery)}" placeholder="工号 / 姓名 / 部门"></label><label class="field">人员类型<select id="prototypeEmployeeType">${["全部类型","正式员工","劳务人员"].map(type => `<option ${type === employeeType ? "selected" : ""}>${type}</option>`).join("")}</select></label>${badge(`本主体 ${prototypeData.employees.length} 人`, "blue")}</div><div class="card"><div class="section-head"><h2>员工档案</h2>${btn("新增员工", "new-employee", true)}</div>${table(["工号", "姓名", "部门", "成本中心", "类型", "入职日期", "固定月薪", "操作"], visible.map(employee => [esc(employee.id),esc(employee.name),esc(employee.department),esc(employee.cost),esc(employee.type),esc(employee.hire),`¥${esc(employee.salary)}`,btn("编辑 / 薪酬方案",`edit-employee-${employee.id}`)]))}${footer}<p class="helper">证件号默认脱敏；一个员工可有多条不冲突的有效薪酬方案。调薪按日期分段计薪。</p></div>`;
};
function downloadSchemaExample(schema){const subject=prototypeData.active||"E-A";const header=schema.fields.map(field=>field[0]);const sample=schema.fields.map(field=>field[0]==="主体ID"?subject:field[3]);const quote=value=>`"${String(value??"").replaceAll('"','""')}"`;const csv=`\uFEFF${header.map(quote).join(",")}\r\n${sample.map(quote).join(",")}\r\n`;const url=URL.createObjectURL(new Blob([csv],{type:"text/csv;charset=utf-8"}));const link=document.createElement("a");link.href=url;link.download=`${schema.name}-示例.csv`;link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
function render(page) {
  current = pages[page] ? page : "overview";
  $("#title").textContent = pages[current][0];
  $("#subtitle").textContent = pages[current][1];
  $("#headingActions").innerHTML = ({subjects:btn("新建主体", "new-subject", true),overview:btn("开始本期核算", "goto-payroll", true),import:btn("下载空白模板", "demo-template"),payroll:btn("重新试算", "demo-calc", true),bonus:btn("试算年终奖", "demo-bonus", true),employee:btn("查看版本对比", "goto-history"),history:btn("导出异常清单", "demo-export"),export:btn("导出计算明细", "demo-export", true)})[current] || "";
  const needsData = ["payroll", "bonus", "employee", "history", "export"].includes(current);
  $("#view").innerHTML = prototypeData.active !== "E-A" && needsData
    ? `<div class="empty"><strong>当前主体暂无${esc(pages[current][0])}数据</strong>先维护员工与工资项目，再导入当期数据。<div style="margin-top:14px">${btn("前往数据导入", "goto-import", true)}</div></div>`
    : panels[current]();
  if (prototypeData.active !== "E-A" && current === "import") {
    const history = document.querySelector("#view .upload-grid + .card");
    if (history) history.innerHTML = `<div class="section-head"><h2>导入历史与错误报告</h2></div><div class="empty"><strong>暂无导入记录</strong>选择上方数据类型并上传文件。</div>`;
  }
  document.querySelectorAll("[data-page]").forEach(el => { el.classList.toggle("active", el.dataset.page === current); if (el.tagName === "BUTTON") el.setAttribute("aria-current", el.dataset.page === current ? "page" : "false"); });
  renderSwitcher();
  if (typeof syncLatestHeading === "function") syncLatestHeading();
}
let toastTimer;
function toast(message) { const el = $("#toast"); el.textContent = message; el.classList.add("show"); clearTimeout(toastTimer); toastTimer = setTimeout(() => el.classList.remove("show"), 2600); }
function prototypeModal(title, body, onSave) {
  $("#prototypeModalTitle").textContent=title;
  $("#prototypeModalBody").innerHTML=body;
  $("#prototypeModal").hidden=false;
  $("#prototypeModalSave").onclick=onSave;
  $("#prototypeModalBody input")?.focus();
}
function closePrototypeModal(){ $("#prototypeModal").hidden=true; }
function handlePrototypeAction(action){
  if(action==="toggle-subject-menu"){setSwitcherOpen($("#entityMenu").hidden);return true;}
  if(action==="view-all-subjects"){setSwitcherOpen(false);render("subjects");return true;}
  if(action.startsWith("choose-subject-")){switchSubject(action.slice(15));return true;}
  if(action.startsWith("schema-")){
    const [,operation,...parts]=action.split("-");const schema=window.SALARY_IMPORT_SCHEMAS.find(entry=>entry.id===parts.join("-"));if(!schema)return true;
    if(operation==="download"){downloadSchemaExample(schema);toast(`已生成 ${schema.name} 示例 CSV`);return true;}
    if(operation==="help"){prototypeModal(`${schema.name} · 字段说明`,`${table(["字段", "要求", "含义", "示例"],schema.fields.map(field=>[esc(field[0]),field[1]?badge("必填","warn"):"可选",esc(field[2]),esc(field[3])]))}<p class="helper">每个文件必须包含当前主体 ID；金额采用人民币十进制，未知值不能伪填 0。</p>`,closePrototypeModal);return true;}
    if(operation==="upload"){prototypeModal(`${schema.name} · 选择与映射`,`<label class="field">选择 XLSX / CSV<input type="file" accept=".xlsx,.csv"></label><div class="flow"><span class="current">选择文件</span><b>›</b><span>字段映射</span><b>›</b><span>行级错误</span></div>${table(["模板字段", "映射来源", "要求"],schema.fields.slice(0,6).map(field=>[esc(field[0]),esc(field[0]),field[1]?"必填":"可选"]))}<p class="helper">完成主体、重复和金额校验后方可写入。</p>`,()=>{closePrototypeModal();toast("请检查字段映射与预检结果");});return true;}
  }
  if(action==="close-prototype-modal"){closePrototypeModal();return true;}
  if(action.startsWith("unlock-")){switchSubject(action.slice(7));return true;}
  if(action==="new-subject"){
    prototypeModal("新建主体工作空间",`<label class="field">主体名称<input id="protoSubjectName" placeholder="企业法定名称"></label><label class="field">主体标识<input id="protoSubjectId" placeholder="例如 E-C"></label><label class="field">税号<input id="protoTax" placeholder="统一社会信用代码"></label><label class="field">地区<input id="protoRegion" placeholder="例如 上海"></label><label class="field">工作空间密码<input id="protoPassword" type="password" placeholder="至少 12 个字符"></label><p class="helper">主体标识创建后不可更改，每个主体使用独立的加密工作空间。</p>`,()=>{const name=$("#protoSubjectName").value.trim(),id=$("#protoSubjectId").value.trim();if(!name||!id)return toast("请填写主体名称和标识");if(prototypeData.subjects.some(s=>s.id===id))return toast("主体标识已存在");prototypeData.subjects.push({id,name,tax:$("#protoTax").value.slice(-4)||"未设置",region:$("#protoRegion").value||"未设置",opened:"刚刚创建"});closePrototypeModal();switchSubject(id);toast("主体已加入列表");});return true;
  }
  if(action==="new-employee"||action.startsWith("edit-employee-")){
    const id=action.startsWith("edit-")?action.slice(14):null,old=prototypeData.employees.find(e=>e.id===id);
    prototypeModal(old?"编辑员工与薪酬方案":"新增员工",`<label class="field">工号<input id="protoEmployeeId" value="${esc(old?.id||"")}" ${old?"readonly":""} placeholder="当前主体内唯一"></label><label class="field">姓名<input id="protoEmployeeName" value="${esc(old?.name||"")}"></label><label class="field">部门<input id="protoDepartment" value="${esc(old?.department||"")}"></label><label class="field">成本中心<input id="protoCost" value="${esc(old?.cost||"")}"></label><label class="field">人员类型<select id="protoEmployeeType"><option>正式员工</option><option>劳务人员</option></select></label><label class="field">入职日期<input id="protoHire" type="date" value="${esc(old?.hire||"")}"></label><label class="field">固定月薪<input id="protoSalary" type="number" min="0" step="0.01" value="${esc(old?.salary||"")}"></label><label class="field">薪酬方案生效日期<input id="protoEffective" type="date" value="2026-01-01"></label>`,()=>{const employee={id:$("#protoEmployeeId").value.trim(),name:$("#protoEmployeeName").value.trim(),department:$("#protoDepartment").value.trim(),cost:$("#protoCost").value.trim(),type:$("#protoEmployeeType").value,hire:$("#protoHire").value,salary:Number($("#protoSalary").value||0).toFixed(2)};if(!employee.id||!employee.name)return toast("请填写工号和姓名");if(!old&&prototypeData.employees.some(e=>e.id===employee.id))return toast("工号重复");if(old)Object.assign(old,employee);else prototypeData.employees.push(employee);closePrototypeModal();render("employees");toast("员工与薪酬方案已保存");});return true;
  }
  if(action==="new-item"){
    prototypeModal("新增工资项目",`<label class="field">项目代码<input id="protoItemCode" placeholder="例如 meal_allowance"></label><label class="field">项目名称<input id="protoItemName" placeholder="例如 餐补"></label><label class="field">类别<select id="protoItemType"><option>收入</option><option>员工扣款</option><option>税后调整</option><option>雇主成本</option></select></label><label class="field">个税属性<select id="protoItemTax"><option>计税</option><option>不计税</option><option>专项扣除</option></select></label><label class="field">社保属性<select id="protoItemSocial"><option>不计入</option><option>计入基数</option></select></label><label class="field">取整节点<select id="protoItemRound"><option>项目到分</option><option>汇总后到分</option></select></label>`,()=>{const code=$("#protoItemCode").value.trim(),name=$("#protoItemName").value.trim();if(!code||!name)return toast("请填写项目代码与名称");if(prototypeData.items.some(item=>item.code===code))return toast("项目代码重复");prototypeData.items.push({code,name,type:$("#protoItemType").value,tax:$("#protoItemTax").value,social:$("#protoItemSocial").value,round:$("#protoItemRound").value});closePrototypeModal();render("items");toast("工资项目已新增");});return true;
  }
  if(action.startsWith("rule-item-")){selectedItem=action.slice(10);render("rules");return true;}
  if(action==="new-rule"||action==="edit-rule"){
    const old=action==="edit-rule"?prototypeData.rules.find(r=>r.item===selectedItem):null;
    prototypeModal(old?"编辑规则为新版本":"手动新增规则",`<label class="field">工资项目<select id="protoRuleItem">${currentItems().map(item=>`<option value="${esc(item.code)}" ${item.code===selectedItem?"selected":""}>${esc(item.name)}</option>`).join("")}</select></label><label class="field">计算公式<input id="protoExpression" value="${esc(old?.expression||"")}" placeholder="例如 月固定薪 ÷ 应计薪天数 × 缺勤天数"></label><label class="field">适用人员<input id="protoScope" value="${esc(old?.scope||"正式员工")}"></label><label class="field">适用地区<input id="protoRuleRegion" value="${esc(old?.region||"上海")}"></label><label class="field">生效起始<input id="protoRuleFrom" type="month" value="2026-01"></label><label class="field">生效结束<input id="protoRuleTo" type="month" value="2026-12"></label><label class="field">制度/政策来源<input id="protoSource" value="${esc(old?.source||"")}"></label><p class="helper">保存后进入待验证状态；需通过样例试算与回算再确认。</p>`,()=>{const item=$("#protoRuleItem").value,expression=$("#protoExpression").value.trim(),source=$("#protoSource").value.trim();if(!expression||!source)return toast("请填写公式和来源");const record={item,expression,scope:$("#protoScope").value,region:$("#protoRuleRegion").value,dates:`${$("#protoRuleFrom").value} 至 ${$("#protoRuleTo").value}`,source,status:"待验证",version:old?"v4":"v1"};if(old)Object.assign(old,record);else prototypeData.rules.push(record);selectedItem=item;closePrototypeModal();render("rules");toast("规则已保存为待验证版本");});return true;
  }
  return false;
}
document.addEventListener("click", e => {
  if (!e.target.closest(".entity-switcher")) setSwitcherOpen(false);
  const pagerButton = e.target.closest("[data-pager-page]");
  if (pagerButton) {
    const state = pagerState.get(pagerButton.dataset.pagerKey);
    if (state && !pagerButton.disabled) { state.page = Number(pagerButton.dataset.pagerPage); render(current); }
    return;
  }
  const page = e.target.closest("[data-page]")?.dataset.page;
  if (page) return render(page);
  const action = e.target.closest("[data-action]")?.dataset.action;
  if (!action) return;
  if (action.startsWith("goto-")) return render(action.slice(5));
  if (handlePrototypeAction(action)) return;
  toast("此操作暂不可用");
});
document.addEventListener("change",e=>{
  if(e.target.id==="prototypeItemSelect"){selectedItem=e.target.value;render("rules");}
  if(e.target.id==="prototypeEmployeeType"){employeeType=e.target.value;pagerState.get("employees").page=1;render("employees");}
  if(e.target.id==="prototypeItemType"){itemType=e.target.value;pagerState.get("items").page=1;render("items");}
  if(e.target.matches("[data-pager-size]")){const state=pagerState.get(e.target.dataset.pagerSize);if(state){state.size=Number(e.target.value);state.page=1;render(current);}}
});
document.addEventListener("input",e=>{
  if(e.target.id!=="prototypeEmployeeSearch")return;
  employeeQuery=e.target.value;
  pagerState.get("employees").page=1;
  const cursor=e.target.selectionStart;
  render("employees");
  const input=$("#prototypeEmployeeSearch");input.focus();input.setSelectionRange(cursor,cursor);
});
document.addEventListener("keydown", e => {
  if (e.key === "Escape" && !$("#entityMenu").hidden) { setSwitcherOpen(false); $("#entityToggle").focus(); }
  if (["ArrowDown", "ArrowUp"].includes(e.key) && !$("#entityMenu").hidden) {
    const options = [...document.querySelectorAll("#entityMenu .entity-option")];
    const index = options.indexOf(document.activeElement);
    options[(index + (e.key === "ArrowDown" ? 1 : -1) + options.length) % options.length]?.focus();
    e.preventDefault();
  }
  if (e.key === "/" && !["INPUT","TEXTAREA"].includes(document.activeElement.tagName)) { e.preventDefault(); $("#prototypeEmployeeSearch")?.focus(); }
});
