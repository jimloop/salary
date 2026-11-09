import { PayrollPage } from './features/payroll/PayrollPage';
import { DateField } from './components/DateField';
import { TaskDialog } from './components/TaskDialog';
import { EmployeesWorkspace } from './features/employees/EmployeesWorkspace';
import { serverKinds } from './features/imports/schemas';
import { ImportsPage } from './features/imports/ImportsPage';
import { importFiles } from './features/imports/importFiles';
// Development-only review entry. Vite's production entry is index.html.
import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { HistoryPage } from './features/history/HistoryPage';
import { WorkspacePeriodProvider } from './components/WorkspacePeriod';
import { api } from './api';
import type { Contribution,ContributionConfiguration,SalaryPlan,Employee,Item,MonthlySnapshot,Rule,Subject } from './types';
import { pageRecords } from './components/records';
import './styles.css';
import './prototype-pages.css';
import './buttons.css';
const subjects:Subject[]=[{id:'E-A',name:'云杉科技',tax_suffix:'3482',region:'上海市',last_opened_at:1793844000},{id:'E-B',name:'海棠咨询',tax_suffix:'9107',region:'杭州市',last_opened_at:null}];
const employees:Employee[]=[{id:'A-1001',name:'林知远',department:'产品部',cost_center:'CC-01',kind:'正式员工',hire_date:'2024-03-01',leave_date:'',base_salary:'10000.00',effective_from:'2026-01-01'}];
const items:Item[]=[{code:'fixed_salary',name:'基本工资',category:'收入',tax_attribute:'应税',social_attribute:'计入基数',rounding:'项目到分'}];
const rules:Rule[]=[{code:'fixed_salary',label:'基本工资',expression:'base_salary / standard_days * actual_days',valid_from:'2026-01-01',valid_to:null,region:'上海市',source:'本主体薪酬制度',version:1,status:'Active'}];
// Prior 10 months: taxable 40000, withheld 1480; month 11 taxable 44000, cumulative tax 1880, withholding 400, net 8600.
const monthly:MonthlySnapshot[]=[{id:'snapshot-0001',subject_id:'E-A',employee_id:'A-1001',period:'2026-10',payout_month:'2026-11',fixed_salary:'10000.00',performance:'0.00',adjustment:'0.00',result:{gross:'10000.00',employee_deductions:'1000.00',withholding:{current_withholding:'400.00',cumulative_taxable_base:'44000.00'},net:'8600.00',employer_cost:'12000.00'},input:{employee_id:'A-1001',period:'2026-10',payout_month:'2026-11',standard_days:'22',actual_days:'22',performance:'0',adjustment:'0',employee_deductions:'1000',employer_contributions:'2000',tax_baseline:{cumulative_income:'100000',cumulative_exempt_income:'0',cumulative_basic_deduction:'50000',cumulative_special_deduction:'10000',cumulative_additional_deduction:'0',cumulative_other_deduction:'0',cumulative_tax_relief:'0',cumulative_withheld_tax:'1480'},basic_deduction:'5000',additional_deduction:'0',other_deduction:'0',exempt_income:'0',tax_relief:'0'},rule_trace:[]}];
const reviewConfigurations:ContributionConfiguration[]=[{id:'review-default',employee_id:null,project:'养老保险',start:'2026-01',end:null,enabled:true,base:'10000',employee_percent:'8',employer_percent:'12'},{id:'review-fund',employee_id:null,project:'住房公积金',start:'2026-01',end:null,enabled:true,base:'10000',employee_percent:'5',employer_percent:'5'}];
const reviewPlans:SalaryPlan[]=[{id:'review-plan',employee_id:'A-1001',effective_from:'2026-01',effective_to:null,base_salary:'10000',note:'虚构核对资料'}];
const reviewContributions=(employeeId='A-1001'):Contribution[]=>reviewConfigurations.filter(r=>r.employee_id===null&&r.enabled).map(r=>{const special=reviewConfigurations.find(v=>v.employee_id===employeeId&&v.project===r.project);const field=(key:'base'|'employee_percent'|'employer_percent')=>({value:special?.[key]??r[key]??'0',source_id:special?.[key]!=null?special.id:r.id,scope:special?.[key]!=null?'employee':'subject'});const base=field('base'),employee_percent=field('employee_percent'),employer_percent=field('employer_percent');return {project:r.project,base,employee_percent,employer_percent,employee_amount:String(Number(base.value)*Number(employee_percent.value)/100),employer_amount:String(Number(base.value)*Number(employer_percent.value)/100)}});

Object.assign(api,{
 deleteContributionConfiguration:async(id:string)=>{const index=reviewConfigurations.findIndex(r=>r.id===id);if(index<0)throw Error('配置不存在');reviewConfigurations.splice(index,1)},
 contributionConfigurations:async()=>[...reviewConfigurations],
 contributionPreview:async(employeeId:string)=>reviewContributions(employeeId),
 contributionStatus:async()=>({configured_count:1,default_count:1,override_count:0,missing_count:0}),
 salaryEffective:async()=>reviewPlans[0],
 saveContributionConfiguration:async(configuration:ContributionConfiguration)=>{const value={...configuration,id:configuration.id||`review-${reviewConfigurations.length}`};const index=reviewConfigurations.findIndex(r=>r.id===value.id);if(index<0)reviewConfigurations.push(value);else reviewConfigurations[index]=value;return value},
 saveSalaryPlan:async(plan:SalaryPlan)=>{const value={...plan,id:plan.id||`salary-${reviewPlans.length}`};const index=reviewPlans.findIndex(p=>p.id===value.id);if(index<0)reviewPlans.push(value);else reviewPlans[index]=value;return value},
 workProgress:async()=>({employee_count:1,import_batch_count:0,default_rule_count:9,custom_rule_count:0,calculated_count:1,version_count:1,stale_count:0,anomaly_count:0,exported_count:0,steps:[{label:'导入与校验',state:'done',detail:'使用已验证输入',target:'imports'},{label:'匹配规则',state:'done',detail:'9 项默认规则',target:'rules'},{label:'工资试算',state:'done',detail:'1 人已有结果',target:'payroll'},{label:'异常检查',state:'done',detail:'未发现异常',target:'history'},{label:'导出',state:'pending',detail:'尚未导出',target:'exports'}],issues:[]}),
 subjects:async()=>subjects,unlock:async()=>{},lock:async()=>{},
 employees:async(page:number,size:number,query:string,kind='')=>pageRecords(employees.filter(e=>(!query||[e.id,e.name,e.department].some(value=>value.includes(query)))&&(!kind||e.kind===kind)),page,size),
 items:async(page:number,size:number,category='')=>pageRecords(items.filter(e=>!category||e.category===category),page,size),
 rules:async(page:number,size:number)=>pageRecords(rules,page,size),payrollVersions:async(page:number,size:number)=>pageRecords(monthly,page,size),
 exportRuns:async()=>[{id:'review-run-1',subject_id:'E-A',period:'2026-10',payout_month:'2026-11',version:1,kind:'batch',created_at:0,version_ids:['snapshot-0001']}],
 bonusSchemes:async()=>[],bonusInputs:async()=>[],bonusVersions:async(page:number,size:number)=>pageRecords([],page,size),importBatches:async(page:number,size:number)=>pageRecords([],page,size),auditPage:async(page:number,size:number)=>pageRecords([],page,size),auditClear:async()=>0,salaryPlans:async()=>pageRecords(reviewPlans,1,20),
});
const reviewBatches:{id:string;subject_id:string;filename:string;sha256:string;sheet:string;kind:string;row_count:number;withdrawn_at:null}[]=[];
importFiles.choose=async()=>'/mock/01-员工基础表.csv';
Object.assign(api,{
 effectiveRules:async()=>rules.map(rule=>({rule,is_default:false,editable:true,inactive_versions:0})),
 importOverview:async()=>Object.values(serverKinds).map(kind=>({kind,row_count:kind==='Employees'?reviewBatches.length:0,batch_count:kind==='Employees'?reviewBatches.length:0,employee_count:kind==='Employees'?reviewBatches.length:0,duplicate_count:0,totals:{}})),
 importData:async(kind:string,_filter:unknown,page:number,size:number)=>pageRecords(kind==='employees'?reviewBatches.map(batch=>({batch_id:batch.id,filename:batch.filename,kind:'Employees',sheet:'CSV',row:2,fields:{employee_id:'MOCK-A-1001',name:'林知远',base_salary:'10000'}})):[],page,size),
 importPreview:async()=>({filename:'01-员工基础表.csv',sha256:'test-hash',sheets:[{name:'CSV',headers:['主体ID','工号','姓名','固定月薪','部门','入职日期','薪酬生效日期'],sample:[['E-A','MOCK-A-1001','林知远','10000','产品部','2024-01-01','2026-01-01']],row_count:1,formula_count:0}]}),
 importValidate:async()=>({total_rows:1,error_count:0,errors:[]}),
 importCommit:async()=>{const value={id:'review-batch',subject_id:'E-A',filename:'01-员工基础表.csv',sha256:'test-hash',sheet:'CSV',kind:'Employees',row_count:1,withdrawn_at:null};reviewBatches.push(value);return value},
 importBatches:async(page:number,size:number)=>pageRecords(reviewBatches,page,size)
});
// Dedicated comparison fixtures keep both modes reviewable without a native session.
const historyPeople=[employees[0],{...employees[0],id:'A-1002',name:'陈亦凡'}];
const historyVersions=historyPeople.flatMap((person,index)=>[2,1].map(version=>({...monthly[0],employee_id:person.id,id:`history-${person.id}-${version}`,performance:index===0&&version===2?'100.00':'0.00',result:{...monthly[0].result,gross:index===0&&version===2?'10100.00':'10000.00',net:index===0&&version===2?'8700.00':'8600.00'}})));
const historyRuns=[2,1].map(version=>({id:`history-run-${version}`,subject_id:'E-A',period:'2026-10',payout_month:'2026-11',version,kind:'batch',created_at:0,version_ids:historyPeople.map(person=>`history-${person.id}-${version}`)}));
const reviewMetrics=(changed:boolean)=>['固定薪','绩效','调整项','应发工资','员工承担','本期预扣个税','实发工资','单位成本'].map((label,i)=>{const before=[10000,0,0,10000,1000,400,8600,12000][i];const delta=changed&&[1,3,6].includes(i)?100:0;return {label,before:String(before),after:String(before+delta),delta:String(delta)}});
const reviewComparison=(employeeId?:string)=>{const people=historyPeople.filter(person=>!employeeId||person.id===employeeId);return {rows:people.map(person=>({employee_id:person.id,name:person.name,status:person.id==='A-1001'?'changed':'same',metrics:reviewMetrics(person.id==='A-1001')})),totals:reviewMetrics(false).map((metric,i)=>{const before=people.length*Number(metric.before);const delta=people.some(person=>person.id==='A-1001')&&[1,3,6].includes(i)?100:0;return {label:metric.label,before:String(before),after:String(before+delta),delta:String(delta),missing_before:0,missing_after:0}})}};
if(location.search==='?history')Object.assign(api,{
 exportRuns:async()=>historyRuns,
 calculationEmployees:async(_period:string,_payout:string,_query:string,page:number,size:number)=>pageRecords(historyPeople.map(person=>({employee_id:person.id,name:person.name,department:person.department,kind:person.kind,version_count:2,latest:historyVersions.find(v=>v.employee_id===person.id)!})),page,size),
 employeeCalculationVersions:async(employeeId:string)=>{const person=historyPeople.find(person=>person.id===employeeId)!;return {employee_id:employeeId,name:person.name,department:person.department,kind:person.kind,versions:historyVersions.filter(v=>v.employee_id===employeeId)}},
 compareRuns:async()=>reviewComparison(),
 compareEmployeeVersions:async(left:string)=>reviewComparison(left.includes('A-1002')?'A-1002':'A-1001'),
});
function EmployeeReview(){const [settings,setSettings]=React.useState(false);return <WorkspacePeriodProvider region="上海市"><main className="content"><h1>员工与薪酬方案</h1><EmployeesWorkspace employees={pageRecords(employees,1,20)} onAdd={()=>{}} onEdit={()=>{}} onPage={()=>{}} onSize={()=>{}} onImport={()=>{}} settingsOpen={settings} onSettings={setSettings}/></main></WorkspacePeriodProvider>}
if(location.search==='?stale')Object.assign(api,{payrollVersions:async(page:number,size:number)=>pageRecords(monthly.map(v=>({...v,needs_recalculation:true})),page,size)});
function DateReview(){const [closed,setClosed]=React.useState(false);const [month,setMonth]=React.useState('2026-10');const [date,setDate]=React.useState('2024-02-01');if(closed)return <p>外层弹框已关闭</p>;return <TaskDialog title="日期控件核对" onClose={()=>setClosed(true)}><div className="form-grid"><label>月份<DateField type="month" value={month} onChange={e=>setMonth(e.target.value)}/></label><label>日期<DateField type="date" value={date} onChange={e=>setDate(e.target.value)}/></label></div></TaskDialog>}
createRoot(document.getElementById('root')!).render(<React.StrictMode>{location.search==='?stale'?<WorkspacePeriodProvider region="上海市"><main className="content"><PayrollPage onHistory={()=>{}} onDetails={()=>{}}/></main></WorkspacePeriodProvider>:location.search==='?dates'?<DateReview/>:location.search==='?employees'?<EmployeeReview/>:location.search==='?imports'?<ImportsPage subject={subjects[0]}/>:location.search==='?history'?<WorkspacePeriodProvider region="上海市"><main className="content"><HistoryPage/></main></WorkspacePeriodProvider>:<App/>}</React.StrictMode>);
