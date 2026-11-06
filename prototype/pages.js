/* UI contract synced with desktop 0.3.6. Sample state is kept inside the prototype. */
const defaultItems = [
  ['fixed_salary','计薪固定薪','收入','计税','按主体制度确认基数','固定月薪 ÷ 标准工作日 × 实出勤日'],
  ['performance_pay','绩效奖金','收入','计税','按主体制度确认基数','绩效奖金输入'],
  ['adjustment_pay','补发补扣','税前调整','计入税前收入','按项目口径确认','补发补扣输入'],
  ['employee_deductions','员工社保公积金承担','员工扣款','专项扣除','员工承担','员工承担输入'],
  ['employer_contributions','单位社保公积金承担','雇主成本','不从员工实发扣除','单位承担','单位承担输入'],
  ['gross','应发合计','汇总','工资薪金收入合计','不重复计入基数','计薪固定薪 + 绩效奖金 + 补发补扣'],
  ['withholding','预估个税','员工扣款','税额测算','不计入','按计划发放月及历史实际税务基线累计预扣测算'],
  ['net','预估实发','汇总','税后实发','不计入','应发合计 − 员工承担 − 预估个税'],
  ['employer_cost','单位总成本','汇总','不作为员工收入重复计税','不计入','应发合计 + 单位承担']
].map(([code,name,type,tax,social,expression])=>({code,name,type,tax,social,expression,round:'项目到分',readonly:['withholding','net','employer_cost'].includes(code)}));
prototypeData.items = defaultItems.map(item=>({...item}));
prototypeData.rules = [];
subjectRecords.set('E-A',{employees:prototypeData.employees,items:prototypeData.items,rules:prototypeData.rules});
let scopePeriod='2026-10', scopePayout='2026-11', scopeRegion='上海', scopeYear='2026';
let monthlyOpen=false, selectedPerson='', personSearch='', calculationPerson='A-1001', calculationVersion='2', calculationQuery='';
const importedKinds=new Set(['employees','salary_plans','attendance','performance','social','tax_history']);
const sampleInput={
  'A-1001':{subject_id:'E-A',employee_id:'A-1001',name:'林知远',department:'产品部',cost_center:'CC-01',hire_date:'2024-03-01',effective_from:'2026-01-01',base_salary:'10000.00',period:'2026-10',standard_days:'22',actual_days:'21',additional_deduction:'0.00',performance:'1000.00',employee_contribution:'800.00',employer_contribution:'1700.00',payout_month:'2026-11',cumulative_income:'100000.00',cumulative_exempt_income:'0.00',cumulative_basic_deduction:'50000.00',cumulative_special_deduction:'8000.00',cumulative_additional_deduction:'0.00',cumulative_other_deduction:'0.00',cumulative_tax_relief:'0.00',cumulative_withheld_tax:'1680.00'},
  'A-1002':{subject_id:'E-A',employee_id:'A-1002',name:'陈亦凡',department:'研发部',cost_center:'CC-02',hire_date:'2025-07-01',effective_from:'2026-01-01',base_salary:'12000.00',period:'2026-10',standard_days:'22',actual_days:'22',additional_deduction:'0.00',performance:'1200.00',employee_contribution:'1000.00',employer_contribution:'2100.00',payout_month:'2026-11',cumulative_income:'120000.00',cumulative_exempt_income:'0.00',cumulative_basic_deduction:'50000.00',cumulative_special_deduction:'10000.00',cumulative_additional_deduction:'0.00',cumulative_other_deduction:'0.00',cumulative_tax_relief:'0.00',cumulative_withheld_tax:'3480.00'}
};
const calculationSamples={
  'A-1001':{fixed:'9545.45',performance:'1000.00',gross:'10545.45',contribution:'800.00',tax:'474.55',net:'9270.90',employer:'1700.00'},
  'A-1002':{fixed:'12000.00',performance:'1200.00',gross:'13200.00',contribution:'1000.00',tax:'720.00',net:'11480.00',employer:'2100.00'}
};
const money=value=>`¥${Number(value).toLocaleString('zh-CN',{minimumFractionDigits:2,maximumFractionDigits:2})}`;
const hasCurrentData=()=>prototypeData.active==='E-A' && scopePeriod==='2026-10' && scopePayout==='2026-11';
const uploaded=schema=>hasCurrentData() && importedKinds.has(schema.id);
const currentItems=()=>[...new Map([...defaultItems,...prototypeData.items].map(item=>[item.code,item])).values()];
const activeRule=code=>prototypeData.rules.find(rule=>rule.item===code && rule.status==='已确认' && rule.dates.slice(0,7)<=scopePeriod && rule.dates.slice(-7)>=scopePeriod && (rule.region===scopeRegion || rule.region==='全国'));
function periodFields(withRegion=false,withYear=false){return `<div class="form-row scope-fields"><label class="field">所属期<input type="month" data-scope="period" value="${scopePeriod}"></label><label class="field">计划发放月<input type="month" data-scope="payout" value="${scopePayout}"></label>${withRegion?`<label class="field">地区<select data-scope="region">${['上海','杭州','全国'].map(region=>`<option ${scopeRegion===region?'selected':''}>${region}</option>`).join('')}</select></label>`:''}${withYear?`<label class="field">奖金年度<input type="number" data-scope="year" value="${scopeYear}"></label>`:''}${badge(`当前主体 ${esc(prototypeData.active)}`,'blue')}</div>`;}
function metrics(entries){return `<div class="cards">${entries.map(([label,value,detail])=>`<div class="card"><div class="metric-label">${label}</div><div class="metric-value">${value}</div><div class="metric-sub">${detail}</div></div>`).join('')}</div>`;}
function progressFlow(){return `<div class="flow">${['导入与校验','匹配规则','工资试算','异常检查','导出'].map((name,i)=>`${i?'<b>›</b>':''}<span class="${hasCurrentData()?(i<4?'done':'current'):(i===0?'current':'')}">${hasCurrentData()&&i<4?'✓':String(i+1).padStart(2,'0')} ${name}</span>`).join('')}</div>`;}
function syncLatestHeading(){
  const canExport=hasCurrentData();
  const actions={overview:btn(hasCurrentData()?'继续导出':'继续导入与校验',hasCurrentData()?'goto-export':'goto-import',true),import:monthlyOpen?btn('返回文件导入','monthly-close'):btn('查询当月上传数据','monthly-open')+btn('下载空白模板','latest-template'),rules:btn('从历史表推断','latest-infer')+btn('导入规则包','latest-pack-import')+btn('导出规则包','latest-pack-export')+btn('手动添加规则','new-rule',true),export:`<button class="btn primary" data-action="demo-export" ${canExport?'':'disabled'}>导出计算明细</button>`};
  if(current in actions) $('#headingActions').innerHTML=actions[current];
  $('#headingActions').classList.add('actions');
  if(current==='import'&&monthlyOpen){$('#title').textContent='当月上传数据';$('#subtitle').textContent='按员工查询当月输入资料，查看上传状态、原始字段与数据汇总。';}
}
panels.overview=()=>{
 const ready=hasCurrentData();
 const stages=[['导入与校验',ready?'6 类输入已写入，行级校验通过':'选择文件、完成字段映射及行级校验后写入','import'],['匹配规则','9 个系统默认项目可用；已确认且适用的自定义规则优先','rules'],['工资试算',ready?'2 人已计算，保留 2 个计算版本':'导入当期输入后进行试算','payroll'],['异常检查',ready?'当前最新结果无阻断异常，可查看逐人逐项解释':'试算后检查缺失资料与异常','history'],['导出',ready?'最新计算版本尚未导出':'完成试算与异常检查后导出','export']];
 return periodFields(true)+metrics([['本期相关导入批次',ready?'6':'0','只统计当前所属期及计划发放月'],['当期可用规则','9','系统默认 9 · 自定义 0'],['本期已试算人数',ready?'2':'0',ready?'2 个版本 · 无待重算结果':'尚未生成本期结果'],['本期核算范围',ready?'2':'0','以当期考勤或手工输入确定']])+`<div class="two-cols"><div class="card"><div class="section-head"><h2>本期工作路径</h2><small>${scopePeriod} → ${scopePayout}</small></div>${progressFlow()}<div class="list">${stages.map(([name,detail,page],i)=>`<div class="list-row"><span class="step-index">${String(i+1).padStart(2,'0')}</span><div><strong>${name}</strong><small>${detail}</small></div><span class="tail">${btn('查看 →',`goto-${page}`)}</span></div>`).join('')}</div></div><div class="card"><div class="section-head"><h2>需要关注</h2><small>当前主体 · 本期</small></div><div class="issue"><span>ⓘ</span><div><strong>${ready?'最新结果待导出':'尚无本期输入'}</strong>${ready?'输入与规则匹配完成，导出时请选择最新计算版本。':'默认规则已经可用，请先导入所属期对应的资料。'}</div></div><p class="helper">输入、规则或有效批次发生变化后，需要重新试算。历史导出记录不代表最新结果已导出。</p></div></div>`;
};
panels.items=()=>{
 const items=currentItems().filter(item=>itemType==='全部项目'||item.type===itemType),{visible,footer}=paginate('items',items,true);
 return `<div class="form-row"><label class="field wide">项目分类<select id="prototypeItemType">${['全部项目','收入','税前调整','员工扣款','雇主成本','汇总'].map(type=>`<option ${itemType===type?'selected':''}>${type}</option>`).join('')}</select></label><label class="field">规则所属期<input type="month" data-scope="period" value="${scopePeriod}"></label>${badge(`${currentItems().length} 个工资项目`,'blue')}</div><div class="card"><div class="section-head"><h2>本主体工资项目</h2>${btn('新增工资项目','new-item',true)}</div>${table(['项目代码','名称','类别','来源','个税属性','社保属性','取整节点','当前规则','操作'],visible.map(item=>[esc(item.code),esc(item.name),esc(item.type),defaultItems.some(d=>d.code===item.code)?'系统默认':'主体配置',esc(item.tax),esc(item.social),esc(item.round),badge(activeRule(item.code)?'自定义规则':'默认规则','blue'),btn(item.readonly?'查看规则':'配置规则',`rule-item-${item.code}`)]))}${footer}<p class="helper">同一代码只展示一个项目。新增收入或补扣项目须在应发合计公式中引用后，才会影响应发金额。</p></div>`;
};
panels.rules=()=>{
 const items=currentItems(),item=items.find(item=>item.code===selectedItem)||items[0],rule=activeRule(item.code);
 return periodFields(true)+`<div class="three-cols"><div class="card"><div class="section-head"><h2>工资项目</h2><small>${items.length} 项</small></div>${items.map(entry=>`<button class="rule-item rule-button ${entry.code===item.code?'active':''}" data-action="rule-item-${entry.code}">${entry.name}<small>${entry.type} · ${activeRule(entry.code)?'自定义规则':'默认规则'}</small></button>`).join('')}</div><div class="card"><div class="section-head"><h2>${esc(item.name)}</h2>${badge(rule?'自定义规则 · '+rule.version:'默认规则','blue')}</div><div class="label">计算公式</div><div class="formula">${esc(rule?.expression||item.expression||'尚未配置规则')}</div><div class="detail"><div><small>适用范围</small><strong>${esc(rule?.scope||'当前主体核算人员')}</strong></div><div><small>有效期</small><strong>${esc(rule?.dates||'系统默认 · 无自定义覆盖时使用')}</strong></div><div><small>地区</small><strong>${esc(rule?.region||'按主体输入')}</strong></div><div><small>取整节点</small><strong>项目到分</strong></div><div><small>来源</small><strong>${esc(rule?.source||'系统默认计算规则')}</strong></div></div>${item.readonly?'<p class="helper">由计算引擎根据输入及税务基线推导，当前项目为只读规则。</p>':`<div class="actions">${btn(rule?'编辑为新版本':'配置自定义规则',rule?'edit-rule':'new-rule')}</div>`}${prototypeData.rules.some(r=>r.item===item.code&&r.status!=='已确认')?'<p class="helper">存在待验证版本；确认生效前继续使用当前适用规则。</p>':''}</div><div class="card"><div class="section-head"><h2>规则使用说明</h2></div><p class="helper">已确认且在所属期、地区范围内有效的自定义规则优先；未配置时使用默认规则。</p><div class="list-row"><div><strong>可追溯的计算</strong><small>结果保留使用的规则、输入值及取整过程。</small></div></div><div class="list-row"><div><strong>规则包独立导入</strong><small>只包含规则定义，导入后成为本主体的待验证版本。</small></div></div><div class="list-row"><div><strong>新增项目需引用</strong><small>自定义收入项目须加入应发合计公式。</small></div></div></div></div>`;
};
function sourceName(schema){return `${schema.id}_${scopePeriod}.csv`;}
function fieldValue(field,person){
 const value=sampleInput[person]?.[field.key];
 if(value===undefined||value==='')return '<span class="missing-value">未提供</span>';
 const unit=/(salary|performance|contribution|deduction|income|tax|adjustment|target|minimum|maximum)/.test(field.key)?' 元':/(days)$/.test(field.key)?' 天':field.key.includes('factor')?' 倍':'';
 return esc(value)+unit;
}
function rawData(schema){
 const people=prototypeData.employees.filter(person=>sampleInput[person.id]);
 return table(schema.fields.map(field=>esc(field.label)),people.map(person=>schema.fields.map(field=>fieldValue(field,person.id))));
}
panels.import=()=>monthlyOpen?monthlyPanel():`<div class="flow"><span class="current">01 选择文件</span><b>›</b><span>02 字段映射</span><b>›</b><span>03 行级校验</span><b>›</b><span>04 写入批次</span></div>${periodFields(false,true)}<div class="upload-grid">${importFields.map(schema=>`<div class="card upload-card"><div class="section-head"><h2>${schema.name}</h2>${badge(schema.stage,'blue')}</div><p class="helper">${schema.fields.length} 个字段 · ${schema.fields.filter(field=>field.required).length} 个必填</p><div class="upload-status">${badge(uploaded(schema)?'已上传':'未上传',uploaded(schema)?'':'neutral')}<span>${uploaded(schema)?'2 条记录 · 1 个有效批次':'当前范围暂无有效批次'}</span></div><div class="actions">${btn('选择文件',`latest-upload-${schema.id}`,true)}${btn('下载示例',`latest-sample-${schema.id}`)}<button class="help-icon" data-action="latest-help-${schema.id}" title="字段说明" aria-label="${schema.name}字段说明">?</button>${uploaded(schema)?btn('查看数据',`latest-data-${schema.id}`):'<button class="btn" disabled>查看数据</button>'}</div></div>`).join('')}</div><div class="card section-gap"><div class="section-head"><h2>导入历史与错误报告</h2><small>仅当前主体</small></div>${hasCurrentData()?table(['文件','类型','行数','结果','操作'],importFields.filter(uploaded).map(schema=>[sourceName(schema),schema.name,'2',badge('已写入'),btn('查看数据',`latest-data-${schema.id}`)])):'<div class="empty">暂无本期导入记录</div>'}<p class="helper">仅有效批次参与当期汇总；历史批次保留来源和行级定位信息。</p></div>`;
function monthlyPanel(){
 const people=prototypeData.employees.filter(person=>[person.id,person.name,person.department].some(value=>value.includes(personSearch))),person=prototypeData.employees.find(person=>person.id===selectedPerson);
 const {visible,footer}=paginate('monthly-people',people,true);
 return periodFields(false,true)+`<div class="three-cols monthly-columns"><div class="card"><div class="section-head"><h2>员工</h2><small>${people.length} 人</small></div><label class="field"><input type="search" id="monthly-person-search" value="${esc(personSearch)}" placeholder="工号 / 姓名 / 部门"></label><button class="rule-item rule-button ${!person?'active':''}" data-action="latest-person-all">全员汇总<small>查看当前主体当月导入资料</small></button>${visible.map(entry=>`<button class="rule-item rule-button ${entry.id===selectedPerson?'active':''}" data-action="latest-person-${entry.id}">${esc(entry.name)}<small>${entry.id} · ${entry.department}</small></button>`).join('')}${footer}</div><div class="card"><div class="section-head"><h2>${person?esc(person.name)+' · 当月详细数据':'全员 · 当月数据汇总'}</h2>${person?btn('查看计算解释','latest-explain-person'):''}</div>${importFields.map(schema=>`<section class="monthly-data-section" id="monthly-${schema.id}"><div class="section-head"><h2>${schema.name}</h2>${badge(uploaded(schema)?'已上传':'未上传',uploaded(schema)?'':'neutral')}</div>${uploaded(schema)?person?`<p class="helper">${sourceName(schema)} · 工作表 1 · 第 ${person.id==='A-1001'?2:3} 行</p><dl class="monthly-fields">${schema.fields.map(field=>`<div><dt>${field.label}</dt><dd>${fieldValue(field,person.id)}</dd></div>`).join('')}</dl>`:`<p class="helper">2 人 · 2 条记录 · 1 个批次</p><p>${schema.id==='attendance'?'实出勤日合计：43 天':schema.id==='performance'?'绩效奖金合计：¥2,200.00':schema.id==='social'?'员工承担合计：¥1,800.00':schema.id==='employees'||schema.id==='salary_plans'?'固定月薪合计：¥22,000.00':'历史实际已预扣个税合计：¥5,160.00'}</p>${btn('查看原始记录',`latest-data-${schema.id}`)}`:'<div class="unuploaded">未上传</div>'}</section>`).join('')}</div><div class="card"><div class="section-head"><h2>各类数据汇总</h2></div>${importFields.map(schema=>`<button class="summary-link" data-action="latest-jump-${schema.id}"><strong>${schema.name}</strong>${badge(uploaded(schema)?'已上传':'未上传',uploaded(schema)?'':'neutral')}<small>${uploaded(schema)?`${person?'1 人 · 1 行':'2 人 · 2 行'}`:'暂无当期资料'}</small></button>`).join('')}</div></div>`;
}
function resultRows(result){return [
 ['计薪固定薪','固定月薪 ÷ 标准工作日 × 实出勤日',result.fixed],['绩效奖金','绩效输入',result.performance],['补发补扣','补发补扣输入','0.00'],['应发合计','计薪固定薪 + 绩效奖金 + 补发补扣',result.gross],['员工承担','员工社保公积金输入',result.contribution],['预估个税','累计预扣测算',result.tax],['预估实发','应发 − 员工承担 − 个税',result.net]];}
panels.employee=()=>{
 const person=prototypeData.employees.find(person=>person.id===calculationPerson)||prototypeData.employees[0];
 if(!hasCurrentData()||!person)return periodFields()+'<div class="empty"><strong>暂无本期计算结果</strong>请先完成当期工资试算。</div>';
 const r=calculationSamples[person.id]||calculationSamples['A-1001'],{visible,footer}=paginate('calculation-people',prototypeData.employees.filter(p=>calculationSamples[p.id]&&[p.id,p.name,p.department].some(value=>value.includes(calculationQuery))),true);
 return periodFields()+`<div class="three-cols"><div class="card"><div class="section-head"><h2>本主体员工</h2><small>${visible.length} 人</small></div><label class="field"><input type="search" id="calculation-person-search" value="${esc(calculationQuery)}" placeholder="工号 / 姓名 / 部门"></label>${visible.map(entry=>`<button class="rule-item rule-button ${entry.id===person.id?'active':''}" data-action="latest-result-person-${entry.id}">${entry.id} · ${esc(entry.name)}<small>${entry.department} · 正式员工 · 2 个版本</small></button>`).join('')}${footer}</div><div class="card"><div class="section-head"><h2>${person.name} · ${scopePeriod} 月薪</h2>${badge(`试算 v${calculationVersion}`,'blue')}</div>${table(['薪资项目','原始值 / 公式','金额'],resultRows(r).map(([name,formula,amount],index)=>[name,`${formula}<small class="table-source">${index===5?'累计预扣测算':'默认规则'}</small>`,`<button class="amount-link" data-action="latest-trace-${index}">${money(amount)}</button>`]))}<div class="label">公式解释</div><div class="formula">${r.gross} − ${r.contribution} − ${r.tax} = ${r.net}</div><p class="helper">点击金额查看原始输入、规则来源和取整过程。</p></div><div class="card"><div class="section-head"><h2>来源与差异</h2></div><label class="field">计算版本<select id="latest-result-version"><option value="2" ${calculationVersion==='2'?'selected':''}>试算 v2 · 最新</option><option value="1" ${calculationVersion==='1'?'selected':''}>试算 v1</option></select></label><div class="detail"><div><small>主体</small><strong>${prototypeData.active}</strong></div><div><small>所属期</small><strong>${scopePeriod}</strong></div><div><small>计划发放月</small><strong>${scopePayout}</strong></div><div><small>舍入</small><strong>项目到分</strong></div></div><div class="label">历史税务资料</div><p class="helper">此前实际已预扣 ${money(sampleInput[person.id].cumulative_withheld_tax)}</p><div class="label">较上一计算版本</div><p class="helper">${calculationVersion==='2'?'应发差额 ¥0.00 · 实发差额 ¥0.00':'当前为首个计算版本，无更早版本。'}</p>${btn('查看版本对比','goto-history')}</div></div>`;
};
panels.payroll=()=>periodFields(true)+progressFlow()+metrics([['本期人数',hasCurrentData()?'2':'0','按本期有效输入'],['应发合计',hasCurrentData()?'¥23,745.45':'—','税前金额'],['预估个税',hasCurrentData()?'¥1,194.55':'—','累计预扣测算'],['预估实发',hasCurrentData()?'¥20,750.90':'—','无阻断异常']])+`<div class="card"><div class="section-head"><h2>员工计算结果</h2>${btn('查看版本对比','goto-history')}</div>${hasCurrentData()?table(['工号','姓名','部门','应发','员工承担','预估个税','预估实发','操作'],prototypeData.employees.filter(p=>calculationSamples[p.id]).map(person=>{const r=calculationSamples[person.id];return [person.id,person.name,person.department,money(r.gross),money(r.contribution),money(r.tax),money(r.net),btn('查看明细',`latest-result-open-${person.id}`)];})):'<div class="empty">尚无当期试算结果</div>'}<p class="helper">重新试算根据当期有效导入批次和适用规则生成独立版本。</p></div>`;
// Match field descriptions and sample downloads to the implemented application's schema.
window.SALARY_IMPORT_SCHEMAS=importFields.map(schema=>({...schema,fields:schema.fields.map(field=>[field.label,field.required,field.description,sampleInput['A-1001'][field.key]||''])}));
const originalPrototypeAction=handlePrototypeAction;
handlePrototypeAction=function(action){
 if(action==='monthly-open'||action==='monthly-close'){monthlyOpen=action==='monthly-open';render('import');return true;}
 if(action.startsWith('latest-person-')){selectedPerson=action.slice(14)==='all'?'':action.slice(14);render('import');return true;}
 if(action.startsWith('latest-result-person-')){calculationPerson=action.slice(21);calculationVersion='2';render('employee');return true;}
 if(action.startsWith('latest-result-open-')){calculationPerson=action.slice(19);calculationVersion='2';render('employee');return true;}
 if(action==='latest-explain-person'){calculationPerson=selectedPerson;calculationVersion='2';render('employee');return true;}
 if(action.startsWith('latest-jump-')){$(`#monthly-${action.slice(12)}`)?.scrollIntoView({behavior:'smooth',block:'start'});return true;}
 if(action.startsWith('latest-trace-')){
  const r=calculationSamples[calculationPerson]||calculationSamples['A-1001'],[name,formula,amount]=resultRows(r)[Number(action.slice(13))];
  const input=sampleInput[calculationPerson]||sampleInput['A-1001'];
  prototypeModal(name+' · 计算解释',`<div class="formula">${formula} = ${money(amount)}</div><div class="detail"><div><small>规则来源</small><strong>系统默认 · 试算 v${calculationVersion}</strong></div><div><small>取整</small><strong>四舍五入到分</strong></div></div><div class="label">原始输入</div>${table(['字段','值'],Number(action.slice(13))===0?[['固定月薪',money(input.base_salary)],['标准工作日',input.standard_days+' 天'],['实出勤日',input.actual_days+' 天']]:Number(action.slice(13))===5?[['此前累计收入',money(input.cumulative_income)],['此前实际已预扣个税',money(input.cumulative_withheld_tax)],['本期预估个税',money(r.tax)]]:Number(action.slice(13))===6?[['应发合计',money(r.gross)],['员工承担',money(r.contribution)],['预估个税',money(r.tax)]]:[['输入 / 计算结果',money(amount)],['来源',`当期有效批次 · ${scopePeriod}`]])}`,closePrototypeModal);$('#prototypeModalSave').textContent='关闭';return true;
 }
 if(action==='latest-template'){prototypeModal('下载空白模板',`<div class="list">${importFields.map(schema=>`<div class="list-row"><strong>${schema.name}</strong><span class="tail">${btn('下载模板',`latest-blank-${schema.id}`)}</span></div>`).join('')}</div>`,closePrototypeModal);$('#prototypeModalSave').textContent='完成';return true;}
 if(action==='latest-infer'){prototypeModal('从历史工资表推断规则',`<label class="field">历史工资表<input type="file" accept=".csv,.xlsx"></label><div class="actions">${btn('下载示例','latest-sample-attendance')}<button class="help-icon" data-action="latest-help-attendance" title="字段说明">?</button></div><div class="flow"><span class="current">选择历史表</span><b>›</b><span>候选规则</span><b>›</b><span>回算差异</span><b>›</b><span>确认规则</span></div><p class="helper">保留公式或提供完整输入资料；推断候选需人工确认，资料不足时不参与计算。</p>`,()=>{prototypeModal('候选规则与历史回算',table(['工资项目','候选公式','匹配','状态'],[['计薪固定薪','固定月薪 ÷ 标准工作日 × 实出勤日','2 / 2 行',badge('待确认','warn')]])+'<p class="helper">差异 ¥0.00；确认后创建本主体的规则版本。</p>',()=>{closePrototypeModal();toast('候选已保存为待验证规则');});});$('#prototypeModalSave').textContent='查看候选';return true;}
 if(action==='latest-pack-import'||action==='latest-pack-export'){prototypeModal(action.endsWith('import')?'导入规则包':'导出规则包',action.endsWith('import')?'<label class="field">选择规则包<input type="file" accept=".json"></label><p class="helper">预览项目、公式和冲突后，创建当前主体的独立待验证版本。</p>':'<p class="helper">规则包仅包含规则定义、适用范围和来源，不包含员工或工资数据。</p>'+table(['工资项目','当前规则'],currentItems().map(item=>[item.name,activeRule(item.code)?'自定义规则':'默认规则'])),closePrototypeModal);$('#prototypeModalSave').textContent=action.endsWith('import')?'预览冲突':'导出';return true;}
 const match=action.match(/^latest-(help|sample|blank|data|upload)-(.+)$/);
 if(match){const [,operation,id]=match,schema=importFields.find(entry=>entry.id===id);if(!schema)return true;
  if(operation==='help'){prototypeModal(schema.name+' · 字段说明',table(['字段','要求','说明'],schema.fields.map(field=>[field.label,field.required?'必填':'选填',field.description])),closePrototypeModal);$('#prototypeModalSave').textContent='关闭';}
  if(operation==='data'){prototypeModal(schema.name+' · 原始记录',rawData(schema),closePrototypeModal);$('#prototypeModalSave').textContent='关闭';}
  if(operation==='sample'||operation==='blank'){
   const quote=value=>'"'+String(value??'').replaceAll('"','""')+'"';
   const rows=[schema.fields.map(field=>field.key)];if(operation==='sample')rows.push(schema.fields.map(field=>sampleInput['A-1001'][field.key]??''));
   const url=URL.createObjectURL(new Blob(['\uFEFF'+rows.map(row=>row.map(quote).join(',')).join('\r\n')],{type:'text/csv;charset=utf-8'})),link=document.createElement('a');link.href=url;link.download=schema.name+(operation==='blank'?'-空白模板':'-示例')+'.csv';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
  }
  if(operation==='upload'){prototypeModal(schema.name+' · 选择文件',`<label class="field">XLSX / CSV<input type="file" id="latest-upload-file" accept=".csv,.xlsx"></label><div id="latest-upload-preview"><p class="helper">选择文件后查看工作表、字段映射与行级校验。</p></div>`,()=>{const file=$('#latest-upload-file')?.files[0];if(!file){toast('请先选择文件');return;}prototypeModal(schema.name+' · 校验与预览',`<div class="flow"><span class="done">✓ 选择文件</span><b>›</b><span class="done">✓ 字段映射</span><b>›</b><span class="current">03 行级校验</span><b>›</b><span>04 写入批次</span></div>${table(['目标字段','来源列','要求'],schema.fields.map(field=>[field.label,field.key,field.required?'必填':'选填']))}<p class="helper">写入前核对主体、所属期、金额与重复记录。</p>`,()=>{importedKinds.add(id);closePrototypeModal();monthlyOpen=false;render('import');toast('已写入输入批次');});$('#prototypeModalSave').textContent='写入批次';});$('#prototypeModalSave').textContent='字段映射与预览';}
  return true;
 }
 $('#prototypeModalSave').textContent='保存';
 return originalPrototypeAction(action);
};
document.addEventListener('change',event=>{
 const el=event.target;
 if(el.dataset.scope){if(el.dataset.scope==='period')scopePeriod=el.value;if(el.dataset.scope==='payout')scopePayout=el.value;if(el.dataset.scope==='region')scopeRegion=el.value;if(el.dataset.scope==='year')scopeYear=el.value;render(current);}
 if(el.id==='prototypeExportFormat')exportFormat=el.value;
 if(el.id==='prototypeExportVersion'){exportVersion=el.value;render('export');}
 if(el.id==='latest-result-version'){calculationVersion=el.value;render('employee');}
 if(el.id==='latest-upload-file'){const preview=$('#latest-upload-preview');preview.innerHTML=el.files[0]?`<div class="upload-status">${badge('待写入','warn')}<span>${esc(el.files[0].name)}</span></div><p class="helper">继续查看字段映射与行级预览后写入。</p>`:'<p class="helper">请选择文件</p>';}
});
document.addEventListener('input',event=>{if(event.target.id==='calculation-person-search'){const cursor=event.target.selectionStart;calculationQuery=event.target.value;render('employee');const input=$('#calculation-person-search');input.focus();input.setSelectionRange(cursor,cursor);}if(event.target.id==='monthly-person-search'){const cursor=event.target.selectionStart;personSearch=event.target.value;render('import');const input=$('#monthly-person-search');input.focus();input.setSelectionRange(cursor,cursor);}});


// Version history keeps snapshots separate from the unique employee list.
panels.history=()=>periodFields()+`<div class="card"><div class="section-head"><h2>版本对比</h2></div><div class="form-row"><label class="field">旧版本<select id="compare-old"><option value="1">试算 v1</option><option value="2">试算 v2 · 最新</option></select></label><label class="field">新版本<select id="compare-new"><option value="2">试算 v2 · 最新</option><option value="1">试算 v1</option></select></label>${btn('比较差异','latest-compare')}</div><div id="version-comparison"></div></div><div class="card section-gap"><div class="section-head"><h2>计算版本</h2><small>2 个版本 · 每版 2 人</small></div>${table(['版本','所属期','计划发放月','人数','应发合计','操作'],['2','1'].map(version=>['试算 v'+version+(version==='2'?' · 最新':''),scopePeriod,scopePayout,'2','¥23,745.45',btn('查看结果',`latest-version-open-${version}`)]))}</div><div class="card section-gap"><div class="section-head"><h2>本期异常</h2></div><div class="empty"><strong>当前结果无阻断异常</strong>输入或规则改变后请重新试算并检查。</div></div>`;
const exportRuns=[
 {version:'2',label:'试算 v2 · 最新 · 2 人',count:2},
 {version:'1',label:'试算 v1 · 2 人',count:2}
];
const exportAudit=[
 ['2026/11/6 09:17:43','解锁主体','workspace'],
 ['2026/11/5 17:40:58','整批计算','2 records'],
 ['2026/11/5 17:36:38','导出工资核算表','1 versions'],
 ['2026/11/5 17:32:32','创建版本','bonuses:c5a'],
 ['2026/11/5 17:28:45','解锁主体','workspace']
];
let exportVersion='2',exportFormat='xlsx';
panels.export=()=>{
 const runs=hasCurrentData()?exportRuns:[];
 const selectedRun=runs.find(run=>run.version===exportVersion)||runs[0];
 const auditPage=paginate('export-audit',exportAudit,true);
 return `<div class="export-stack"><section class="card export-result-card">
  <div class="section-head"><h2>计算结果导出</h2>${badge('仅当前主体','blue')}</div>
  <div class="export-fields">
   <label class="field">所属期<input type="month" data-scope="period" value="${scopePeriod}"></label>
   <label class="field">计划发放月<input type="month" data-scope="payout" value="${scopePayout}"></label>
   <label class="field">试算版本<select id="prototypeExportVersion" ${runs.length?'':'disabled'}>${runs.length?runs.map(run=>`<option value="${run.version}" ${run.version===selectedRun.version?'selected':''}>${run.label}</option>`).join(''):'<option>暂无计算版本</option>'}</select></label>
   <label class="field">文件内容<select id="prototypeExportFormat"><option value="xlsx" ${exportFormat==='xlsx'?'selected':''}>工资核算 XLSX（5 张表）</option><option value="csv" ${exportFormat==='csv'?'selected':''}>员工明细 CSV</option></select></label>
  </div>
  <div class="export-file-summary"><strong>导出文件将包含</strong><small>主体、所属期、计划发放月、版本、生成时间、仅供薪酬计算使用</small></div>
  <p class="helper export-version-summary">${selectedRun?`将导出试算 v${selectedRun.version} 的 ${selectedRun.count} 名员工结果。`:'当前期间暂无可导出的计算结果。'}</p>
  <p class="helper">导出前需再次验证当前主体密码；缺失的税额与实发会明确标记为不可准确测算。</p>
 </section><section class="card export-backup-card"><div class="section-head"><h2>单主体离线备份</h2><small>不包含其他主体</small></div><p class="helper">备份文件包含当前主体的加密资料与完整性校验。恢复前验证主体与文件完整性；不同主体或损坏文件将拒绝写入。</p><div class="actions export-backup-actions">${btn('创建加密备份','latest-backup',true)}${btn('从备份恢复','latest-restore')}</div><div class="export-audit"><h3>操作日志</h3><p class="helper">记录当前主体的资料变更、导入与计算操作。</p>${table(['时间','操作','对象'],auditPage.visible)}${auditPage.footer}</div></section></div>`;
};
const latestPageAction=handlePrototypeAction;
handlePrototypeAction=function(action){
 if(action==='latest-compare'){const same=$('#compare-old').value===$('#compare-new').value;$('#version-comparison').innerHTML=same?'<p class="helper">请选择两个不同的计算版本。</p>':table(['员工','应发差额','个税差额','实发差额'],prototypeData.employees.filter(person=>calculationSamples[person.id]).map(person=>[person.name,'¥0.00','¥0.00','¥0.00']));return true;}
 if(action.startsWith('latest-version-open-')){calculationVersion=action.slice(20);render('employee');return true;}
 if(action==='demo-export'){const run=exportRuns.find(value=>value.version===exportVersion);prototypeModal('导出计算明细',`<div class="detail"><div><small>试算版本</small><strong>${run?.label||'暂无计算版本'}</strong></div><div><small>文件内容</small><strong>${exportFormat==='xlsx'?'工资核算 XLSX（5 张表）':'员工明细 CSV'}</strong></div></div><label class="field">当前主体<strong>${esc(prototypeData.subjects.find(subject=>subject.id===prototypeData.active)?.name)}</strong></label><label class="field">工作空间密码<input type="password" placeholder="输入当前主体密码"></label>`,closePrototypeModal);$('#prototypeModalSave').textContent='导出';return true;}
 if(action==='latest-backup'||action==='latest-restore'){prototypeModal(action==='latest-backup'?'创建加密备份':'从备份恢复',`<label class="field">当前主体<strong>${esc(prototypeData.subjects.find(subject=>subject.id===prototypeData.active)?.name)}</strong></label><label class="field">工作空间密码<input type="password" placeholder="输入当前主体密码"></label>${action==='latest-restore'?'<label class="field">备份文件<input type="file"></label>':''}`,closePrototypeModal);$('#prototypeModalSave').textContent=action==='latest-restore'?'恢复':'创建备份';return true;}
 if(action==='demo-calc'){prototypeModal('重新试算',`<div class="detail"><div><small>所属期</small><strong>${scopePeriod}</strong></div><div><small>计划发放月</small><strong>${scopePayout}</strong></div></div><p class="helper">根据当期有效导入批次和匹配规则生成新版本，历史结果保留。</p>`,()=>{closePrototypeModal();render('payroll');toast('试算结果已生成');});$('#prototypeModalSave').textContent='开始试算';return true;}
 return latestPageAction(action);
};
render('overview');
