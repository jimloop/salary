import { useEffect,useState } from 'react';
import { api } from '../../api';
import { ComboSelect } from '../../components/ComboSelect';
import { Pagination } from '../../components/Pagination';
import { pageRecords } from '../../components/records';
import type { CalculationRun,RunComparison as Report } from '../../types';

import { comparisonLabels as statuses,metricTone } from './comparisonStyle';
const money=(value:string|null)=>value===null?'不可准确测算':`¥${Number(value).toLocaleString('zh-CN',{minimumFractionDigits:2,maximumFractionDigits:2})}`;
export function RunComparison({period,payoutMonth,refresh}:{period:string;payoutMonth:string;refresh:number}){
 const [runs,setRuns]=useState<CalculationRun[]>([]);const [left,setLeft]=useState('');const [right,setRight]=useState('');
 const [report,setReport]=useState<Report|null>(null);const [loading,setLoading]=useState(false);const [error,setError]=useState('');
 const [filter,setFilter]=useState<keyof typeof statuses|'all'>('all');const [query,setQuery]=useState('');const [page,setPage]=useState(1);const [size,setSize]=useState(20);
 useEffect(()=>{
  let active=true;setRuns([]);setLeft('');setRight('');setReport(null);setError('');
  void api.exportRuns(period,payoutMonth).then(value=>{if(active){setRuns(value);setLeft(value[1]?.id??'');setRight(value[0]?.id??'')}}).catch(reason=>{if(active)setError(String(reason))});
  return()=>{active=false};
 },[period,payoutMonth,refresh]);
 useEffect(()=>{
  let active=true;setReport(null);setLoading(false);setError('');setPage(1);
  if(left&&right&&left!==right){setLoading(true);void api.compareRuns(left,right).then(value=>{if(active)setReport(value)}).catch(reason=>{if(active)setError(String(reason))}).finally(()=>{if(active)setLoading(false)});}
  return()=>{active=false};
 },[left,right]);
 const data=pageRecords((report?.rows??[]).filter(row=>(filter==='all'||row.status===filter)&&(!query||`${row.name} ${row.employee_id}`.includes(query))),page,size);
 return <>
  <section className="panel"><div className="panel-head"><div><h2>全员工资对比</h2><p>逐人比较各工资项目，并统计两个版本的项目合计。差额为新版本减原版本。</p></div></div>
   <div className="prototype-form-row">{([['原版本',left,setLeft],['新版本',right,setRight]] as const).map(([label,value,setValue])=><label key={label}>{label}<ComboSelect value={value} onChange={event=>setValue(event.target.value)}><option value="">选择试算版本</option>{runs.map(run=><option key={run.id} value={run.id}>试算 v{run.version} · {run.version_ids.length} 人{run.kind==='single'?' · 单人':''}</option>)}</ComboSelect></label>)}</div>
   {error&&<div className="form-error" role="alert">{error}</div>}
   {loading&&<div className="import-progress">正在逐人核对工资项目与统计合计…</div>}
   {!loading&&!report&&<p className="prototype-helper">{runs.length<2?'当前期间至少需要两个试算版本才能对比。':'请选择两个不同的试算版本。'}</p>}
   {report&&<><h3>工资项目合计</h3><div className="table-wrap"><table><thead><tr><th>工资项目</th><th className="number">原版本合计</th><th className="number">新版本合计</th><th className="number">差额</th></tr></thead><tbody>{report.totals.map(metric=><tr key={metric.label} className={`comparison-row comparison-${metricTone(metric)}`}><td>{metric.label}</td><td className="number">{metric.missing_before?'已知合计 ':''}{money(metric.before)}{metric.missing_before>0&&<small> · 缺 {metric.missing_before} 人</small>}</td><td className="number">{metric.missing_after?'已知合计 ':''}{money(metric.after)}{metric.missing_after>0&&<small> · 缺 {metric.missing_after} 人</small>}</td><td className="number comparison-value">{money(metric.delta)}</td></tr>)}</tbody></table></div></>}
  </section>
  {report&&<section className="panel"><div className="panel-head"><h2>员工工资明细对比</h2><small>共 {report.rows.length} 人</small></div>
   <div className="comparison-filters"><button aria-pressed={filter==='all'} className={filter==='all'?'selected':''} onClick={()=>{setFilter('all');setPage(1)}}>全部 {report.rows.length}</button>{Object.entries(statuses).map(([key,label])=><button key={key} aria-pressed={filter===key} className={filter===key?'selected':''} onClick={()=>{setFilter(key as keyof typeof statuses);setPage(1)}}>{label} {report.rows.filter(row=>row.status===key).length}</button>)}</div>
   <input className="employee-data-search" type="search" aria-label="搜索对比员工" placeholder="姓名 / 工号" value={query} onChange={event=>{setQuery(event.target.value);setPage(1)}}/>
   <div className="table-wrap"><table className="comparison-employee-table"><thead><tr><th>姓名</th><th>工号</th><th>对比结果</th><th className="number">应发差额</th><th className="number">实发差额</th><th>差异项目</th></tr></thead><tbody>{data.records.map(row=><tr key={row.employee_id} className={`comparison-row comparison-${row.status}`}><td>{row.name}</td><td>{row.employee_id}</td><td><span className={`comparison-status comparison-${row.status}`}>{statuses[row.status]}</span></td><td className="number comparison-value">{row.status==='added'||row.status==='removed'?'—':money(row.metrics[3].delta)}</td><td className="number comparison-value">{row.status==='added'||row.status==='removed'?'—':money(row.metrics[6].delta)}</td><td><details><summary>{row.status==='added'||row.status==='removed'?'查看工资数据':row.status==='incomplete'?'已知项目一致，部分数据缺失':row.metrics.filter(metric=>metricTone(metric)==='changed').map(metric=>metric.label).join('、')||'各工资项目一致'}</summary><table className="comparison-details"><thead><tr><th>项目</th><th>原版本</th><th>新版本</th><th>差额</th></tr></thead><tbody>{row.metrics.map(metric=><tr key={metric.label} className={`comparison-row comparison-${metricTone(metric)}`}><td>{metric.label}</td><td>{row.status==='added'?'—':money(metric.before)}</td><td>{row.status==='removed'?'—':money(metric.after)}</td><td className="comparison-value">{row.status==='added'||row.status==='removed'?'—':money(metric.delta)}</td></tr>)}</tbody></table></details></td></tr>)}</tbody></table>{!data.total&&<div className="table-empty">没有符合筛选条件的员工。</div>}</div>
   <Pagination data={data} onPage={setPage} onSize={value=>{setSize(value);setPage(1)}}/>
   <p className="prototype-helper">“一致”表示各项工资金额完整且相同。个税、实发等存在缺失时单独标记，不把未知金额按零计算。</p>
  </section>}
 </>;
}
