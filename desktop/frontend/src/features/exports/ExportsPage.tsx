import { ComboSelect } from '../../components/ComboSelect';
import { DateField } from '../../components/DateField';
import { useWorkspacePeriod } from '../../components/WorkspacePeriod';
import { PageAction } from '../../components/PageAction';
import { TaskDialog } from '../../components/TaskDialog';
import { AuditPanel } from '../history/AuditPanel';
import { useEffect,useRef,useState } from 'react';
import { open,save } from '@tauri-apps/plugin-dialog';
import { api } from '../../api';
import type { CalculationRun, Subject } from '../../types';

export function ExportsPage({subject,onLock,onFeedback}:{subject:Subject;onLock:()=>void;onFeedback:(text:string)=>void}){
  const {period,setPeriod,payoutMonth,setPayoutMonth}=useWorkspacePeriod();
  const [auth,setAuth]=useState<'xlsx'|'csv'|'backup'|'restore'|null>(null);
  const [format,setFormat]=useState<'xlsx'|'csv'>('xlsx');
  const [runs,setRuns]=useState<CalculationRun[]>([]);
  const [selectedRunId,setSelectedRunId]=useState('');
  const [loadingRuns,setLoadingRuns]=useState(false);
  const [password,setPassword]=useState('');
  const [confirmedName,setConfirmedName]=useState('');
  const [error,setError]=useState('');
  const [busy,setBusy]=useState(false);const submitting=useRef(false);
  const [queryRefresh,setQueryRefresh]=useState(0);
  const [auditRefresh,setAuditRefresh]=useState(0);

  useEffect(()=>{
    let active=true;
    setLoadingRuns(true);setRuns([]);setSelectedRunId('');setError('');
    void api.exportRuns(period,payoutMonth).then(value=>{
      if(active){setRuns(value);setSelectedRunId(value[0]?.id??'');}
    }).catch(reason=>{if(active)setError(String(reason));}).finally(()=>{if(active)setLoadingRuns(false);});
    return()=>{active=false};
  },[subject.id,period,payoutMonth,queryRefresh]);

  const selectedRun=runs.find(value=>value.id===selectedRunId&&value.period===period&&value.payout_month===payoutMonth);
  const run=async(work:()=>Promise<string>)=>{
    if(submitting.current)return;submitting.current=true;setBusy(true);setError('');
    try{onFeedback(await work());setAuditRefresh(value=>value+1);setAuth(null);setPassword('');}
    catch(reason){setError(String(reason));}
    finally{submitting.current=false;setBusy(false);}
  };
  const exportCsv=()=>run(async()=>{
    if(!selectedRun)throw new Error('请先选择试算版本');
    const path=await save({defaultPath:`${subject.name}-${selectedRun.period.replace('-','年')}月-v${selectedRun.version}-员工明细.csv`,filters:[{name:'工资明细 CSV',extensions:['csv']}]});
    if(!path)return '已取消导出';
    const count=await api.exportPayrollCsv(path,password,selectedRun.id);
    return `已导出试算 v${selectedRun.version} 的 ${count} 名员工结果`;
  });
  const exportXlsx=()=>run(async()=>{
    if(!selectedRun)throw new Error('请先选择试算版本');
    const path=await save({defaultPath:`${subject.name}-${selectedRun.period.replace('-','年')}月-v${selectedRun.version}-工资核算.xlsx`,filters:[{name:'工资核算 XLSX',extensions:['xlsx']}]});
    if(!path)return '已取消导出';
    const count=await api.exportPayrollXlsx(path,password,selectedRun.id);
    return `已导出试算 v${selectedRun.version} 的 ${count} 名员工结果（5 张工作表）`;
  });
  const backup=()=>run(async()=>{
    const path=await save({defaultPath:`${subject.name}-加密备份.salarybackup`,filters:[{name:'薪酬加密备份',extensions:['salarybackup']}]});
    if(!path)return '已取消备份';
    await api.backupExport(path,password);
    return '单主体加密备份已保存';
  });
  const restore=()=>run(async()=>{
    const path=await open({multiple:false,filters:[{name:'薪酬加密备份',extensions:['salarybackup']}]});
    if(typeof path!=='string')return '已取消恢复';
    await api.lock();onLock();
    await api.backupRestore(path,subject.id,password,confirmedName);
    return '备份已恢复，请重新解锁工作空间';
  });

  const confirmationDisabled=busy||!password||(auth==='restore'&&confirmedName!==subject.name)||((auth==='xlsx'||auth==='csv')&&!selectedRun);
  const confirmAuth=()=>{if(confirmationDisabled)return;void (auth==='xlsx'?exportXlsx:auth==='csv'?exportCsv:auth==='backup'?backup:restore)()};
  return <>
    <PageAction><button className="primary" disabled={!selectedRun||busy} onClick={()=>setAuth(format)}>导出计算明细</button></PageAction>
    <div className="exports-stack">
      <section className="panel">
        <div className="panel-head"><h2>计算结果导出</h2><span className="badge">仅当前主体</span></div>
        <div className="exports-fields">
          <label>所属期<DateField type="month" value={period} onChange={event=>setPeriod(event.target.value)}/></label>
          <label>计划发放月<DateField type="month" value={payoutMonth} onChange={event=>setPayoutMonth(event.target.value)}/></label>
          <label>试算版本<ComboSelect value={selectedRun?.id??''} disabled={!runs.length} onChange={event=>setSelectedRunId(event.target.value)}>
            {!runs.length&&<option value="">{loadingRuns?'正在读取…':'暂无试算版本'}</option>}
            {runs.map((value,index)=><option key={value.id} value={value.id}>试算 v{value.version}{index===0?' · 最新':''} · {value.version_ids.length} 人{value.kind==='single'?' · 单人':''}{value.needs_recalculation?' · 需要重新计算':''}</option>)}
          </ComboSelect></label>
          <label>文件内容<ComboSelect value={format} onChange={event=>setFormat(event.target.value as 'xlsx'|'csv')}><option value="xlsx">工资核算 XLSX（5 张表）</option><option value="csv">员工明细 CSV</option></ComboSelect></label>
        </div>
        <div className="query-actions"><button disabled={loadingRuns} onClick={()=>setQueryRefresh(value=>value+1)}>重新查询</button></div>
        <div className="prototype-list-row"><div><strong>导出文件将包含</strong><small>工号、姓名、工资项目及实际计算金额；主体、所属年月和版本体现在文件名中</small></div></div>
        <p className="prototype-helper">{selectedRun?`将导出试算 v${selectedRun.version} 的 ${selectedRun.version_ids.length} 名员工结果。`:loadingRuns?'正在读取试算版本…':'当前期间暂无可导出的试算版本，请先重新试算。'}</p>
        <p className="prototype-helper">导出前需再次验证当前主体密码；缺失的税额与实发会明确标记为不可准确测算。</p>
      </section>
      <section className="panel">
        <div className="panel-head"><h2>单主体离线备份</h2><small>不包含其他主体</small></div>
        <p className="prototype-helper">备份文件包含当前主体的加密资料与完整性校验。恢复前验证主体与文件完整性；不同主体或损坏文件将拒绝写入。</p>
        <div className="backup-actions"><button className="primary" onClick={()=>setAuth('backup')}>创建加密备份</button><button onClick={()=>setAuth('restore')}>从备份恢复</button></div>
        <div className="embedded-audit"><AuditPanel refresh={auditRefresh}/></div>
      </section>
    </div>
    {error&&!auth&&<div className="form-error" role="alert">{error}</div>}
    {auth&&<TaskDialog title={auth==='restore'?'恢复主体备份':auth==='backup'?'创建加密备份':'验证并导出'} onClose={()=>{setAuth(null);setPassword('');setError('')}}>
      {(auth==='xlsx'||auth==='csv')&&selectedRun&&<p className="prototype-helper">试算 v{selectedRun.version} · {selectedRun.version_ids.length} 名员工 · {auth==='xlsx'?'工资核算 XLSX（5 张表）':'员工明细 CSV'}</p>}
      <div className="form-grid"><label className="wide">当前主体密码<input type="password" disabled={busy} onKeyDown={event=>{if(event.key==='Enter'&&!event.nativeEvent.isComposing&&!event.repeat){event.preventDefault();confirmAuth()}}} autoComplete="off" value={password} onChange={event=>setPassword(event.target.value)}/></label>{auth==='restore'&&<label className="wide">输入主体名称确认<input value={confirmedName} onChange={event=>setConfirmedName(event.target.value)} placeholder={subject.name}/></label>}</div>
      {error&&<div className="form-error" role="alert">{error}</div>}
      <div className="dialog-actions"><button onClick={()=>setAuth(null)}>取消</button><button className="primary" disabled={confirmationDisabled} onClick={confirmAuth}>{busy?'处理中…':'验证并继续'}</button></div>
    </TaskDialog>}
  </>;
}
