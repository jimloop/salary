import { RegionSelect } from './components/RegionSelect';
import { ComboSelect } from './components/ComboSelect';
import { DateField } from './components/DateField';
import { ItemsPage } from './features/items/ItemsPage';
import { WorkspacePeriodProvider } from './components/WorkspacePeriod';
import { OverviewPage } from './features/workspace/OverviewPage';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from './api';
import { Pagination } from './components/Pagination';
import { PayrollPage } from './features/payroll/PayrollPage';
import { ImportsPage } from './features/imports/ImportsPage';
import { RulesWorkspace } from './features/rules/RulesWorkspace';
import { BonusPage } from './features/bonus/BonusPage';
import { EmployeeCalculationPage } from './features/results/EmployeeCalculationPage';
import { HistoryPage } from './features/history/HistoryPage';
import { ExportsPage } from './features/exports/ExportsPage';
import type { Employee, SalaryPlan, Item, Page, Rule, Section, Subject } from './types';

const labels: Record<Section, string> = { subjects:'主体列表', overview:'工作台', employees:'员工与薪酬方案', items:'工资项目', rules:'规则配置', imports:'数据导入', payroll:'月薪计算', bonus:'年终奖计算', results:'员工计算明细', history:'版本与异常', exports:'导出与备份' };
const icons: Record<Section,string> = { subjects:'▦', overview:'◫', employees:'♙', items:'▤', rules:'ƒ', imports:'⇧', payroll:'≋', bonus:'◇', results:'♙', history:'◷', exports:'⇩' };
const navSections:Section[]=['overview','employees','items','imports','rules','payroll','bonus','results','history','exports'];
const emptyEmployee: Employee = { id:'',name:'',department:'',cost_center:'',kind:'正式员工',hire_date:'',leave_date:'',base_salary:'',effective_from:'' };
const emptyItem: Item = { code:'',name:'',category:'收入',tax_attribute:'应税',social_attribute:'不计入',rounding:'最终金额保留两位' };
const emptyRule: Rule = { code:'',label:'',expression:'',valid_from:'',valid_to:null,region:null,source:'内部制度',version:1,status:'Draft' };
type Modal = { kind:'create'|'unlock'|'delete'|'employee'|'plan'|'item'|'rule'; subject?:Subject; employee?:Employee; item?:Item; rule?:Rule } | null;
const errorText = (error: unknown) => typeof error === 'string' ? error : error instanceof Error ? error.message : '操作失败';

function App() {
  const [subjects,setSubjects] = useState<Subject[]>([]);
  const [subject,setSubject] = useState<Subject|null>(null);
  const [unlocked,setUnlocked] = useState(false);
  const [section,setSection] = useState<Section>('subjects');
  const [menu,setMenu] = useState(false);
  const subjectControlRef=useRef<HTMLDivElement>(null);
  const [modal,setModal] = useState<Modal>(null);
  const [message,setMessage] = useState('');
  const [page,setPage] = useState(1);
  const [pageSize,setPageSize] = useState(20);
  const [subjectPage,setSubjectPage] = useState(1);
  const [subjectSize,setSubjectSize] = useState(20);
  const [query,setQuery] = useState('');const [employeeKind,setEmployeeKind]=useState('');const [employeeRefresh,setEmployeeRefresh]=useState(0);const [itemRefresh,setItemRefresh]=useState(0);const [ruleItem,setRuleItem]=useState('');const [detailEmployee,setDetailEmployee]=useState('');
  const [employees,setEmployees] = useState<Page<Employee>>({records:[],total:0,page:1,page_size:20});
  const [items,setItems] = useState<Page<Item>>({records:[],total:0,page:1,page_size:20});
  const [rules,setRules] = useState<Page<Rule>>({records:[],total:0,page:1,page_size:20});
  const refreshSubjects = useCallback(async () => { try { const list=await api.subjects(); setSubjects(list); setSubject(current => current ? list.find(s=>s.id===current.id)??null : list[0]??null); return list; } catch(e) { setMessage(errorText(e)); return [] as Subject[]; } },[]);
  useEffect(()=>{void refreshSubjects();},[refreshSubjects]);
  useEffect(()=>{if(!menu)return;const closeOutside=(event:MouseEvent)=>{if(!subjectControlRef.current?.contains(event.target as Node))setMenu(false)};const closeEscape=(event:KeyboardEvent)=>{if(event.key==='Escape')setMenu(false)};document.addEventListener('mousedown',closeOutside);document.addEventListener('keydown',closeEscape);return()=>{document.removeEventListener('mousedown',closeOutside);document.removeEventListener('keydown',closeEscape)}},[menu]);
  useEffect(()=>{ setPage(1); setQuery('');setEmployeeKind(''); },[section,subject?.id]);
  useEffect(()=>{
    if(!unlocked) return;
    let active=true;
    const load= async()=>{try{
      if(section==='employees'){ const value=await api.employees(page,pageSize,query,employeeKind); if(active)setEmployees(value); }
      if(section==='rules'){const [configured,wages]=await Promise.all([api.rules(page,pageSize),api.items(1,100)]);if(active){setRules(configured);setItems(wages);}}
    }catch(e){if(active)setMessage(errorText(e));}};
    void load();return()=>{active=false};
  },[section,subject?.id,unlocked,page,pageSize,query,employeeKind,employeeRefresh]);
  const refreshPage=async()=>{ try { if(section==='employees')setEmployees(await api.employees(page,pageSize,query,employeeKind)); if(section==='items')setItemRefresh(value=>value+1); if(section==='rules')setRules(await api.rules(page,pageSize)); }catch(e){setMessage(errorText(e));} };
  const selectSubject=(target:Subject)=>{setMenu(false);if(subject?.id===target.id&&unlocked){setSection('subjects');return;}setModal({kind:'unlock',subject:target});};
  const changeSize=(size:number)=>{setPageSize(size);setPage(1);};
  const closeModal=()=>setModal(null);
  const effectiveSubjectPage=Math.min(subjectPage,Math.max(1,Math.ceil(subjects.length/subjectSize)));
  const subjectList:Page<Subject>={records:subjects.slice((effectiveSubjectPage-1)*subjectSize,effectiveSubjectPage*subjectSize),total:subjects.length,page:effectiveSubjectPage,page_size:subjectSize};

  return <WorkspacePeriodProvider key={subject?.id??'none'} region={subject?.region??''}><div className="shell">
    <aside className="sidebar">
      <div className="product">薪酬计算助手</div>
      <div className="side-label">当前工作空间</div>
      <div className="subject-control" ref={subjectControlRef}><button className="subject-button" onClick={()=>setMenu(!menu)} aria-haspopup="menu" aria-expanded={menu}><span className="avatar">{subject?.name.slice(0,1)??'+'}</span><span><strong>{subject?.name??'新建主体'}</strong><small>{subject?`税号末位 · ${subject.tax_suffix}`:'创建独立工作空间'}</small></span><span className="chevron">⌄</span></button>
      {menu&&<div className="subject-menu" role="menu" aria-label="选择主体"><div className="subject-menu-title">切换主体</div>{subjects.map(s=><button key={s.id} role="menuitemradio" aria-checked={s.id===subject?.id} onClick={()=>selectSubject(s)}><span className="avatar small">{s.name.slice(0,1)}</span><span>{s.name}<small>主体标识 {s.id.slice(0,8)} · 税号末位 {s.tax_suffix}</small></span>{s.id===subject?.id&&<b>✓</b>}</button>)}<button className="menu-create" role="menuitem" onClick={()=>{setMenu(false);setSection('subjects');}}>查看全部主体 →</button></div>}</div>
      <div className={'workspace-state '+(unlocked?'ok':'')}><span className="state-dot" />{unlocked?'已解锁 · 仅当前主体可见':'工作空间已锁定'}</div>
      <div className="search-box"><span>⌕</span><input aria-label="搜索当前主体" placeholder="搜索当前主体的员工、批次或规则" value={query} onChange={e=>{setQuery(e.target.value);setPage(1);}} /><kbd>/</kbd></div>
      <div className="side-label nav-label">工作流程</div><nav>{navSections.map(key=><button key={key} className={section===key?'active':''} onClick={()=>setSection(key)}><span>{icons[key]}</span>{labels[key]}</button>)}</nav>
      <div className="sidebar-foot"><strong>本地离线工作</strong><p>薪酬资料保存在当前主体的加密工作空间。</p>{unlocked&&<button className="text-button" onClick={async()=>{await api.lock();setUnlocked(false);setMessage('工作空间已锁定');}}>锁定工作空间</button>}</div>
    </aside>
    <main className="main"><div className="content"><div className="heading"><div><div className="eyebrow">PAYROLL WORKSPACE</div><h1>{labels[section]}</h1><p>{({subjects:'选择并解锁一个主体，或创建新的独立工作空间。',overview:'聚焦当前主体的导入、规则和计算状态。',employees:'维护当前主体的人员、部门、成本中心和有效薪酬方案。',items:'先定义收入、扣款和成本项目，再逐项配置计算规则。',rules:'每条规则都有适用范围、有效期、来源和独立版本。',imports:'先核对主体与字段，再决定是否写入本主体的输入批次。',payroll:'从输入完整度到逐项解释，清晰完成当期试算。',bonus:'查看资格、预算与两种计税方式的模拟结果。',results:'追溯原始值、规则版本、取整步骤与期间差异。',history:'保留每次计算快照，定位变化与阻断问题。',exports:'导出当前主体计算结果，管理单主体离线备份。'} as Record<Section,string>)[section]}</p></div><div id="page-actions" className="page-actions"/>{section==='subjects'&&<button className="primary heading-action" onClick={()=>setModal({kind:'create'})}>新建主体</button>}</div>
      {section==='subjects' ? <><div className="section-title">本机工作空间</div>{subjects.length===0?<div className="empty"><h2>还没有主体工作空间</h2><p>新建主体后即可录入员工、工资项目与计算规则。</p><button className="primary" onClick={()=>setModal({kind:'create'})}>新建主体</button></div>:<><div className="subject-grid">{subjectList.records.map(s=><div className="subject-tile" key={s.id}><div className="subject-tile-top"><span className="avatar">{s.name.slice(0,1)}</span><span><strong>{s.name}</strong><small>主体标识 {s.id.slice(0,8)} · 税号末位 {s.tax_suffix} · {s.region||'地区未设置'}</small></span></div><div className="subject-tile-bottom"><span>{s.last_opened_at?`上次打开 ${new Date(s.last_opened_at*1000).toLocaleString('zh-CN')}`:'尚未打开'}</span><div><button onClick={()=>selectSubject(s)}>{s.id===subject?.id&&unlocked?'当前主体':'切换到此主体'}</button><button className="danger-text" onClick={()=>setModal({kind:'delete',subject:s})}>删除</button></div></div></div>)}</div><Pagination data={subjectList} onPage={setSubjectPage} onSize={size=>{setSubjectSize(size);setSubjectPage(1);}}/></>}<p className="workflow-hint">切换主体后，员工、工资项目和规则仅显示所选主体的数据。</p></>
      : !subject ? <div className="empty"><h2>请先新建主体</h2><p>从当前工作空间选择器进入主体列表。</p><button className="primary" onClick={()=>setSection('subjects')}>查看主体列表</button></div>
      : !unlocked ? <div className="empty"><h2>{subject.name} 已锁定</h2><p>输入工作空间密码后查看此主体资料。</p><button className="primary" onClick={()=>setModal({kind:'unlock',subject})}>解锁工作空间</button></div>
      : <>
        {section==='overview'&&<OverviewPage onNavigate={setSection}/>}
        {section==='employees'&&<div className="list-toolbar"><label>搜索员工<input type="search" placeholder="工号 / 姓名 / 部门" value={query} onChange={event=>{setQuery(event.target.value);setPage(1)}}/></label><label>人员类型<ComboSelect value={employeeKind} onChange={e=>{setEmployeeKind(e.target.value);setPage(1)}}><option value="">全部类型</option><option>正式员工</option><option>劳务人员</option></ComboSelect></label><button className="query-button" onClick={()=>setEmployeeRefresh(value=>value+1)}>重新查询</button><span className="badge">本主体 {employees.total} 人</span></div>}
        {section==='employees'&&<div className="panel"><div className="panel-head"><div><h2>员工档案</h2><p>工号在当前主体内唯一；薪酬方案按生效日期保存。</p></div><button className="primary" onClick={()=>setModal({kind:'employee',employee:{...emptyEmployee}})}>新增员工</button></div><div className="table-wrap"><table><thead><tr><th>工号</th><th>姓名</th><th>部门</th><th>成本中心</th><th>类型</th><th>入职日期</th><th className="number">固定月薪</th><th>操作</th></tr></thead><tbody>{employees.records.map(e=><tr key={e.id}><td>{e.id}</td><td>{e.name}</td><td>{e.department}</td><td>{e.cost_center}</td><td>{e.kind}</td><td>{e.hire_date}</td><td className="number">¥{e.base_salary}</td><td><button onClick={()=>setModal({kind:'employee',employee:e})}>编辑 / 薪酬方案</button></td></tr>)}</tbody></table>{employees.total===0&&<div className="table-empty">尚无员工。点击“新增员工”建立档案。</div>}</div><Pagination data={employees} onPage={setPage} onSize={changeSize}/></div>}
        {section==='items'&&<ItemsPage refresh={itemRefresh} onAdd={()=>setModal({kind:'item',item:{...emptyItem}})} onEdit={item=>setModal({kind:'item',item})} onRule={code=>{setRuleItem(code);setSection('rules')}}/>}
        {section==='rules'&&<RulesWorkspace defaultRegion={subject.region} initialCode={ruleItem} items={items} rules={rules} onAdd={()=>setModal({kind:'rule',rule:{...emptyRule}})} onEdit={rule=>setModal({kind:'rule',rule})} onRefresh={()=>void refreshPage()} onPage={setPage} onSize={changeSize}/>}
        {section==='payroll'&&<PayrollPage defaultRegion={subject.region} onHistory={()=>setSection('history')} onDetails={employeeId=>{setDetailEmployee(employeeId);setSection('results')}}/>}
        {section==='imports'&&<ImportsPage subject={subject} onViewCalculation={employeeId=>{setDetailEmployee(employeeId);setSection('results')}}/>}
        {section==='bonus'&&<BonusPage/>}
        {section==='results'&&<EmployeeCalculationPage initialEmployee={detailEmployee} onHistory={()=>setSection('history')}/>}
        {section==='history'&&<HistoryPage/>}
        {section==='exports'&&<ExportsPage subject={subject} onLock={()=>setUnlocked(false)} onFeedback={setMessage}/>}
      </>}
    </div></main>
    {message&&<div className="toast" role="status" onClick={()=>setMessage('')}>{message}<button aria-label="关闭提示">×</button></div>}
    {modal&&<Dialog key={`${modal.kind}-${modal.employee?.id??modal.subject?.id??'new'}`} onOpenPlans={employee=>setModal({kind:'plan',employee})} modal={modal} onClose={closeModal} onLock={()=>{setUnlocked(false);setEmployees({records:[],total:0,page:1,page_size:20});setItems({records:[],total:0,page:1,page_size:20});setRules({records:[],total:0,page:1,page_size:20});}} onSuccess={async(text,opened)=>{closeModal();setMessage(text);const latest=await refreshSubjects();if(opened){setSubject(latest.find(s=>s.id===opened.id)??opened);setUnlocked(true);setSection('overview');}if(!['delete','unlock','create'].includes(modal.kind))await refreshPage();}} />}
  </div></WorkspacePeriodProvider>;
}

function Dialog({modal,onClose,onLock,onSuccess,onOpenPlans}:{onOpenPlans:(employee:Employee)=>void;modal:NonNullable<Modal>;onClose:()=>void;onLock:()=>void;onSuccess:(message:string,opened?:Subject)=>void}) {
  const [name,setName]=useState(''); const [taxSuffix,setTaxSuffix]=useState(''); const [region,setRegion]=useState(''); const [password,setPassword]=useState(''); const [confirmName,setConfirmName]=useState('');
  const [employee,setEmployee]=useState<Employee>(modal.employee??emptyEmployee); const [item,setItem]=useState<Item>(modal.item??emptyItem); const [rule,setRule]=useState<Rule>(modal.rule??emptyRule);
  const [plans,setPlans]=useState<SalaryPlan[]>([]);const [plan,setPlan]=useState<SalaryPlan>({id:'',employee_id:modal.employee?.id??'',effective_from:'',effective_to:null,base_salary:modal.employee?.base_salary??'',note:''});
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  useEffect(()=>{if(modal.kind==='plan'&&modal.employee?.id){void api.salaryPlans(modal.employee.id,1,100).then(page=>setPlans(page.records)).catch(e=>setError(errorText(e)));}},[modal.kind,modal.employee?.id]);
  const submit=async()=>{setBusy(true);setError('');try{
    if(modal.kind==='create'){const created=await api.createSubject(name,taxSuffix,region,password);await api.unlock(created.id,password);onSuccess('主体已创建并解锁',created);}
    if(modal.kind==='unlock'&&modal.subject){await api.lock();onLock();await api.unlock(modal.subject.id,password);onSuccess('工作空间已解锁',modal.subject);}
    if(modal.kind==='delete'&&modal.subject){await api.lock();onLock();await api.deleteSubject(modal.subject.id,password,confirmName);onSuccess('主体及应用内数据已删除');}
    if(modal.kind==='employee'){await api.saveEmployee(employee);onSuccess('员工档案已保存');}
    if(modal.kind==='plan'){await api.saveSalaryPlan(plan);onSuccess('薪酬方案已保存');}
    if(modal.kind==='item'){await api.saveItem(item);onSuccess('工资项目已保存');}
    if(modal.kind==='rule'){await api.saveRule(rule);onSuccess('规则已保存');}
  }catch(e){setError(errorText(e));}finally{setBusy(false);}};
  const title={create:'新建主体',unlock:`解锁 ${modal.subject?.name}`,delete:`删除 ${modal.subject?.name}`,employee:modal.employee?.id?'编辑员工档案':'新增员工',plan:`${modal.employee?.name} · 薪酬方案`,item:modal.item?.code?'编辑工资项目':'新增工资项目',rule:modal.rule?.code?'配置规则':'手动添加规则'}[modal.kind];
  return <div className="modal-backdrop" onMouseDown={onClose}><div className="dialog" role="dialog" aria-modal="true" aria-label={title} onMouseDown={e=>e.stopPropagation()}><div className="dialog-head"><h2>{title}</h2><button aria-label="关闭" onClick={onClose}>×</button></div>
    <div className="form-grid">
      {modal.kind==='create'&&<><label>主体名称<input value={name} onChange={e=>setName(e.target.value)} autoFocus /></label><label>统一社会信用代码末四位<input value={taxSuffix} maxLength={4} onChange={e=>setTaxSuffix(e.target.value)} /></label><label>地区<RegionSelect value={region} onChange={e=>setRegion(e.target.value)}/></label></>}
      {['create','unlock','delete'].includes(modal.kind)&&<label>工作空间密码<input type="password" value={password} onChange={e=>setPassword(e.target.value)} autoFocus={modal.kind!=='create'}/></label>}
      {modal.kind==='delete'&&<><p className="danger-note">将删除此主体在应用内的员工、规则、导入、计算、版本和日志。自行导出的文件不受影响。</p><label>输入主体名称确认<input value={confirmName} onChange={e=>setConfirmName(e.target.value)} /></label></>}
      {modal.kind==='employee'&&<><label>工号<input value={employee.id} disabled={!!modal.employee?.id} onChange={e=>setEmployee({...employee,id:e.target.value})}/></label><label>姓名<input value={employee.name} onChange={e=>setEmployee({...employee,name:e.target.value})}/></label><label>部门<input value={employee.department} onChange={e=>setEmployee({...employee,department:e.target.value})}/></label><label>成本中心<input value={employee.cost_center} onChange={e=>setEmployee({...employee,cost_center:e.target.value})}/></label><label>人员类型<ComboSelect value={employee.kind} onChange={e=>setEmployee({...employee,kind:e.target.value})}><option>正式员工</option><option>非全日制</option><option>实习生</option></ComboSelect></label><label>入职日期<DateField type="date" value={employee.hire_date} onChange={e=>setEmployee({...employee,hire_date:e.target.value})}/></label><label>离职日期<DateField type="date" value={employee.leave_date} onChange={e=>setEmployee({...employee,leave_date:e.target.value})}/></label><label>固定月薪<input type="number" min="0" step="0.01" value={employee.base_salary} onChange={e=>setEmployee({...employee,base_salary:e.target.value})}/></label><label>薪酬生效日期<DateField type="date" value={employee.effective_from} onChange={e=>setEmployee({...employee,effective_from:e.target.value})}/></label></>}
      {modal.kind==='plan'&&<><div className="wide existing-plans"><strong>已有方案</strong><p>基础档案固定月薪 ¥{modal.employee?.base_salary}，单独方案按生效日期覆盖。月中调薪须提供分段考勤。</p>{plans.map(existing=><button key={existing.id} onClick={()=>setPlan(existing)}>{existing.effective_from} 至 {existing.effective_to??'长期'} · ¥{existing.base_salary} · 编辑</button>)}</div><label>生效日期<DateField type="date" value={plan.effective_from} onChange={e=>setPlan({...plan,effective_from:e.target.value})}/></label><label>结束日期<DateField type="date" value={plan.effective_to??''} onChange={e=>setPlan({...plan,effective_to:e.target.value||null})}/></label><label>固定月薪<input type="number" min="0" step="0.01" value={plan.base_salary} onChange={e=>setPlan({...plan,base_salary:e.target.value})}/></label><label>备注<input value={plan.note} onChange={e=>setPlan({...plan,note:e.target.value})}/></label></>}
      {modal.kind==='item'&&<><label>项目编码<input value={item.code} disabled={!!modal.item?.code} onChange={e=>setItem({...item,code:e.target.value})}/></label><label>项目名称<input value={item.name} onChange={e=>setItem({...item,name:e.target.value})}/></label><label>类别<ComboSelect value={item.category} onChange={e=>setItem({...item,category:e.target.value})}><option>收入</option><option>员工扣款</option><option>税前调整</option><option>税后调整</option><option>雇主成本</option><option>汇总</option></ComboSelect></label><label>个税属性<ComboSelect value={item.tax_attribute} onChange={e=>setItem({...item,tax_attribute:e.target.value})}><option>应税</option><option>免税</option><option>税前扣除</option></ComboSelect></label><label>社保属性<ComboSelect value={item.social_attribute} onChange={e=>setItem({...item,social_attribute:e.target.value})}><option>不计入</option><option>计入基数</option></ComboSelect></label><label>舍入节点<input value={item.rounding} onChange={e=>setItem({...item,rounding:e.target.value})}/></label></>}
      {modal.kind==='rule'&&<><label>工资项目编码<input value={rule.code} disabled={!!modal.rule?.code} onChange={e=>setRule({...rule,code:e.target.value})}/></label><label>规则名称<input value={rule.label} onChange={e=>setRule({...rule,label:e.target.value})}/></label><label className="wide">公式（例：base_salary / standard_days * actual_days）<input value={rule.expression} onChange={e=>setRule({...rule,expression:e.target.value})}/></label><label>生效日期<DateField type="date" value={rule.valid_from} onChange={e=>setRule({...rule,valid_from:e.target.value})}/></label><label>截止日期<DateField type="date" value={rule.valid_to??''} onChange={e=>setRule({...rule,valid_to:e.target.value||null})}/></label><label>适用地区<RegionSelect value={rule.region??''} onChange={e=>setRule({...rule,region:e.target.value||null})} emptyLabel="全部地区"/></label><label>制度或政策来源<input value={rule.source} onChange={e=>setRule({...rule,source:e.target.value})}/></label><label>状态<ComboSelect value={rule.status} onChange={e=>setRule({...rule,status:e.target.value as Rule['status']})}><option value="Draft">草稿</option><option value="PendingValidation">待验证</option><option value="Active">已确认</option></ComboSelect></label></>}
    </div>{error&&<div className="form-error" role="alert">{error}</div>}<div className="dialog-actions">{modal.kind==='employee'&&modal.employee?.id&&<button onClick={()=>onOpenPlans(modal.employee!)}>管理薪酬方案</button>}<button onClick={onClose}>取消</button><button className={modal.kind==='delete'?'danger':'primary'} onClick={submit} disabled={busy}>{busy?'处理中…':modal.kind==='delete'?'确认删除':'保存'}</button></div></div></div>;
}
export default App;
