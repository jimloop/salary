import { useEffect, useState } from 'react';
import { api } from '../../api';
import { Pagination } from '../../components/Pagination';
import { TaskDialog } from '../../components/TaskDialog';
import type { AuditEntry, Page } from '../../types';

const empty: Page<AuditEntry> = { records: [], total: 0, page: 1, page_size: 20 };
const actionName: Record<string,string> = {
  unlock: '解锁主体', upsert: '保存资料', insert: '创建版本',
  calculate_batch: '整批计算', import_rule_pack: '导入规则包', import_file: '导入文件',
  export_payroll: '导出工资表', backup_export: '导出备份', backup_restore: '恢复备份',
  export_payroll_xlsx: '导出工资核算表',
  withdraw_import: '撤销导入批次', restore_import: '恢复导入批次',
};

function targetLabel(target:string){try{const value=JSON.parse(target);if(typeof value.run_version==='number'){const count=value.employee_count??value.version_ids?.length;return `试算 v${value.run_version}${count?` · ${count} 人`:''}`;}if(Array.isArray(value.version_ids))return `${value.version_ids.length} 个员工结果`;}catch{/* Earlier log entries use plain text. */}return target;}

export function AuditPanel({refresh=0}:{refresh?:number}) {
  const [data,setData]=useState(empty);
  const [page,setPage]=useState(1);
  const [size,setSize]=useState(20);
  const [error,setError]=useState('');
  const [confirmClear,setConfirmClear]=useState(false);
  const [clearing,setClearing]=useState(false);
  const [revision,setRevision]=useState(0);
  useEffect(()=>{let active=true;void api.auditPage(page,size).then(value=>{if(active){setData(value);setError('');}}).catch(reason=>{if(active)setError(String(reason));});return()=>{active=false};},[page,size,revision,refresh]);
  const clear=async()=>{
    setClearing(true);setError('');
    try{await api.auditClear();setConfirmClear(false);setPage(1);setRevision(value=>value+1);}
    catch(reason){setError(String(reason));}
    finally{setClearing(false);}
  };
  return <div className="panel"><div className="panel-head"><div><h2>操作日志</h2><p>记录当前主体的资料变更、导入与计算操作。</p></div><div className="audit-actions"><button onClick={()=>setRevision(value=>value+1)}>重新查询</button><button className="danger-text" disabled={!data.total||clearing} onClick={()=>setConfirmClear(true)}>清空历史日志</button></div></div>
    <div className="table-wrap"><table><thead><tr><th>时间</th><th>操作</th><th>对象</th></tr></thead><tbody>{data.records.map(row=><tr key={row.id}><td>{new Date(row.timestamp*1000).toLocaleString('zh-CN')}</td><td>{actionName[row.action]??row.action}</td><td>{targetLabel(row.target)}</td></tr>)}</tbody></table>{data.total===0&&<div className="table-empty">暂无操作记录。</div>}</div>
    <Pagination data={data} onPage={setPage} onSize={value=>{setSize(value);setPage(1);}}/>
    {error&&<div className="form-error" role="alert">{error}</div>}
    {confirmClear&&<TaskDialog title="清空历史日志" onClose={()=>{if(!clearing)setConfirmClear(false)}}><p className="prototype-helper">将永久删除当前主体的 {data.total} 条操作日志。其他业务数据不受影响。</p><div className="dialog-actions"><button disabled={clearing} onClick={()=>setConfirmClear(false)}>取消</button><button className="danger" disabled={clearing} onClick={clear}>{clearing?'正在清空…':'确认清空'}</button></div></TaskDialog>}
  </div>;
}
