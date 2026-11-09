import { ConfigurationChanges } from './ConfigurationChanges';
import { useEffect,useState } from 'react';
import { ComboSelect } from '../../components/ComboSelect';
import { DateField } from '../../components/DateField';
import { Pagination } from '../../components/Pagination';
import { loadAllRecords,pageRecords } from '../../components/records';
import { useWorkspacePeriod } from '../../components/WorkspacePeriod';
import { api } from '../../api';
import { RunComparison } from './RunComparison';
import { comparisonLabels,metricTone } from './comparisonStyle';
import type { CalculationEmployee,CalculationRun,EmployeeVersions,MonthlySnapshot,RunComparison as ComparisonReport } from '../../types';

export function HistoryPage(){
 const {period,setPeriod,payoutMonth,setPayoutMonth}=useWorkspacePeriod();
 const [mode,setMode]=useState<'all'|'single'>('all');
 const [people,setPeople]=useState<CalculationEmployee[]>([]);
 const [employeeId,setEmployeeId]=useState('');
 const [details,setDetails]=useState<EmployeeVersions|null>(null);
 const [runs,setRuns]=useState<CalculationRun[]>([]);
 const [page,setPage]=useState(1);const [size,setSize]=useState(20);
 const [left,setLeft]=useState('');const [right,setRight]=useState('');
 const [diff,setDiff]=useState<ComparisonReport|null>(null);
 const [error,setError]=useState('');const [loading,setLoading]=useState(false);
 const [comparing,setComparing]=useState(false);const [refresh,setRefresh]=useState(0);
 const [compareRefresh,setCompareRefresh]=useState(0);
 useEffect(()=>{
  let active=true;setPeople([]);setError('');if(mode!=='single')return;
  void loadAllRecords((p,s)=>api.calculationEmployees(period,payoutMonth,'',p,s)).then(value=>{
   if(active){setPeople(value);setEmployeeId(current=>value.some(person=>person.employee_id===current)?current:value[0]?.employee_id??'');}
  }).catch(reason=>{if(active)setError(String(reason))});
  return()=>{active=false};
 },[period,payoutMonth,refresh,mode]);
 useEffect(()=>{
  let active=true;setDetails(null);setRuns([]);setLeft('');setRight('');setDiff(null);setPage(1);setLoading(!!employeeId&&mode==='single');if(mode!=='single')return;
  if(employeeId)void Promise.all([api.employeeCalculationVersions(employeeId,period,payoutMonth),api.exportRuns(period,payoutMonth)]).then(([value,runList])=>{
   if(active){setDetails(value);setRuns(runList);setLeft(value.versions[1]?.id??'');setRight(value.versions[0]?.id??'');}
  }).catch(reason=>{if(active)setError(String(reason))}).finally(()=>{if(active)setLoading(false)});
  return()=>{active=false};
 },[employeeId,period,payoutMonth,refresh,mode]);
 useEffect(()=>{
  let active=true;setDiff(null);setComparing(false);setError('');
  if(mode==='single'&&left&&right&&left!==right){setComparing(true);void api.compareEmployeeVersions(left,right).then(value=>{if(active)setDiff(value)}).catch(reason=>{if(active)setError(String(reason))}).finally(()=>{if(active)setComparing(false)});}
  return()=>{active=false};
 },[left,right,compareRefresh,mode]);
 const versions=details?.versions??[];
 const data=pageRecords(versions,page,size);
 const name=details?.name??people.find(person=>person.employee_id===employeeId)?.name??'';
 const versionLabel=(value:MonthlySnapshot)=>{
  const run=runs.find(run=>run.version_ids.includes(value.id));
  return `试算 v${run?.version??versions.length-versions.indexOf(value)}${value.id===versions[0]?.id?' · 最新':''}${value.needs_recalculation?' · 需要重新计算':''}`;
 };
 const money=(value:string|null|undefined)=>value==null?'不可准确测算':`¥${Number(value).toLocaleString('zh-CN',{minimumFractionDigits:2,maximumFractionDigits:2})}`;
 const issues=versions.filter(value=>!value.result.withholding||(value.result.net!==null&&Number(value.result.net)<0));
 return <div className="history-page">{mode==='single'&&diff&&<ConfigurationChanges rows={diff.rows}/>}
  <div className="calculation-modes" role="group" aria-label="对比范围"><button aria-pressed={mode==='all'} className={mode==='all'?'active':''} onClick={()=>setMode('all')}>全员对比</button><button aria-pressed={mode==='single'} className={mode==='single'?'active':''} onClick={()=>setMode('single')}>单员工对比</button></div>
  <div className="prototype-form-row">
   <label>所属期<DateField type="month" value={period} onChange={event=>setPeriod(event.target.value)}/></label>
   <label>计划发放月<DateField type="month" value={payoutMonth} onChange={event=>setPayoutMonth(event.target.value)}/></label>
   {mode==='single'&&<label>选择员工<ComboSelect value={employeeId} disabled={!people.length} onChange={event=>setEmployeeId(event.target.value)}><option value="">选择员工</option>{people.map(person=><option key={person.employee_id} value={person.employee_id}>{person.name} · {person.employee_id}</option>)}</ComboSelect></label>}
   <button className="primary query-button" disabled={loading} onClick={()=>setRefresh(value=>value+1)}>重新查询</button>
  </div>
  {mode==='all'?<RunComparison period={period} payoutMonth={payoutMonth} refresh={refresh}/>:<>
  {error&&<div className="form-error" role="alert">{error}</div>}
  <div className="prototype-two-cols"><section className="panel">
   <div className="panel-head"><h2>{name?`${name} · 计算版本`:'员工计算版本'}</h2><small>{employeeId?`${employeeId} · ${versions.length} 个版本`:'先选择员工，再比较版本'}</small></div>
   {loading?<div className="import-progress">正在读取员工版本…</div>:<><div className="table-wrap"><table><thead><tr><th>姓名</th><th>工号</th><th>试算版本</th><th>应发工资</th><th>实发工资</th></tr></thead><tbody>{data.records.map(value=><tr key={value.id}><td>{name}</td><td>{value.employee_id}</td><td>{versionLabel(value)}</td><td>{money(value.result.gross)}</td><td>{money(value.result.net)}</td></tr>)}</tbody></table>{!data.total&&<div className="table-empty">所选期间暂无员工计算版本。</div>}</div><Pagination data={data} onPage={setPage} onSize={value=>{setSize(value);setPage(1)}}/></>}
   <div className="rule-label">版本对比{name?` · ${name}`:''} {diff?.rows[0]&&<span className={`comparison-status comparison-${diff.rows[0].status}`}>{comparisonLabels[diff.rows[0].status]}</span>}</div>
   <div className="prototype-form-row">{([['原版本',left,setLeft],['新版本',right,setRight]] as const).map(([label,value,setValue])=><label key={label}>{label}<ComboSelect value={value} disabled={!versions.length} onChange={event=>setValue(event.target.value)}><option value="">选择版本</option>{versions.map(version=><option key={version.id} value={version.id}>{versionLabel(version)}</option>)}</ComboSelect></label>)}<button className="primary query-button" disabled={!left||!right||left===right||comparing} onClick={()=>setCompareRefresh(value=>value+1)}>重新比较</button></div>
   {comparing?<div className="import-progress">正在比较所选版本…</div>:diff?<div className="table-wrap"><table><thead><tr><th>工资项目</th><th>原版本</th><th>新版本</th><th>差额</th></tr></thead><tbody>{diff.rows[0]?.metrics.map(metric=><tr key={metric.label} className={`comparison-row comparison-${metricTone(metric)}`}><td>{metric.label}</td><td>{money(metric.before)}</td><td>{money(metric.after)}</td><td className="comparison-value">{money(metric.delta)}</td></tr>)}</tbody></table></div>:<p className="prototype-helper">{versions.length<2?'同一员工在所选期间至少需要两个计算版本才能对比。':'请选择两个不同版本，选择后自动比较差异。'}</p>}
  </section><section className="panel"><div className="panel-head"><h2>{name?`${name} · 异常清单`:'异常清单'}</h2><span className="badge">{issues.length} 项</span></div>{issues.map(value=><div className="prototype-issue danger" key={value.id}><b>!</b><div><strong>{name} · {!value.result.withholding?'历史实际税额缺失':'预估实发为负'}</strong><p>{!value.result.withholding?'请补充完整的历史收入、扣除和实际已预扣税额后重新试算。':'请核对原始补扣款和计算规则。'}</p><small>{value.employee_id} · {versionLabel(value)}</small></div></div>)}{!issues.length&&<p className="prototype-helper">所选员工在当前期间没有已识别的异常。</p>}</section></div>
 </>}
 </div>;
}
