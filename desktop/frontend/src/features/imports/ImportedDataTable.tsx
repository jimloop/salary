import { useEffect,useState } from 'react';
import { api } from '../../api';
import { Pagination } from '../../components/Pagination';
import type { DataFilter,ImportedDataRow,ImportKind,Page } from '../../types';
import { fieldLabel,schemas } from './schemas';
export function ImportedDataTable({kind,filter}:{kind:ImportKind;filter:DataFilter}){
 const [page,setPage]=useState(1);const [size,setSize]=useState(20);const [data,setData]=useState<Page<ImportedDataRow>>({records:[],total:0,page:1,page_size:20});const [busy,setBusy]=useState(false);const [error,setError]=useState('');
 const key=JSON.stringify([kind,filter.period,filter.payout_month,filter.year,filter.employee_id,filter.batch_id]);
 useEffect(()=>setPage(1),[key]);
 useEffect(()=>{let active=true;setBusy(true);setData({records:[],total:0,page,page_size:size});setError('');void api.importData(kind,filter,page,size).then(value=>{if(active)setData(value)}).catch(reason=>{if(active)setError(String(reason))}).finally(()=>{if(active)setBusy(false)});return()=>{active=false}},[key,page,size]);
 const fields=[...new Set([...schemas[kind].fields.map(([field])=>field),...data.records.flatMap(row=>Object.keys(row.fields))])].filter(field=>field!=='subject_id');
 return <div aria-busy={busy}>{kind==='social'&&data.records.some(row=>!row.fields.project&&row.fields.employee_contribution!==undefined)&&<p className="not-ready">旧版金额批次 · 仅保留历史记录，不参与新计算。</p>}{busy&&<div className="import-progress" role="status">正在读取导入数据…</div>}{error?<div className="form-error">{error}</div>:<><div className="table-wrap"><table><thead><tr>{fields.map(field=><th key={field}>{fieldLabel(field)}</th>)}<th>来源文件</th><th>工作表 / 行</th></tr></thead><tbody>{data.records.map(row=><tr key={`${row.batch_id}-${row.row}`}>{fields.map(field=><td key={field}>{row.fields[field]==null||row.fields[field]===''?'未提供':row.fields[field]}</td>)}<td>{row.filename}</td><td>{row.sheet} / {row.row}</td></tr>)}</tbody></table>{!busy&&!data.total&&<div className="table-empty">所选范围暂无导入数据。</div>}</div><Pagination data={data} onPage={setPage} onSize={value=>{setSize(value);setPage(1)}}/></>}{!filter.batch_id&&<p className="prototype-helper">显示当前生效的导入资料；撤销批次不参与汇总。空值显示“未提供”，不按零处理。</p>}</div>;
}
