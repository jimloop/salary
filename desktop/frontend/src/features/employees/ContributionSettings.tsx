import { useState } from 'react';
import { api } from '../../api';
import { TaskDialog } from '../../components/TaskDialog';
import { ComboSelect } from '../../components/ComboSelect';
import { DateField } from '../../components/DateField';
import type { ContributionConfiguration } from '../../types';
const projects=['养老保险','医疗保险','失业保险','工伤保险','生育保险','住房公积金'];
export function ContributionSettings({records,employeeId=null,period,onSaved,onClose,initialProject}:{initialProject?:string;records:ContributionConfiguration[];employeeId?:string|null;period:string;onSaved:()=>void;onClose:()=>void}){
 const employeeAdd=employeeId!==null&&!initialProject;
 const employeeEdit=employeeId!==null&&!!initialProject;
 const effective=(row:ContributionConfiguration,month=period)=>row.start<=month&&(!row.end||row.end>=month);
 const rows=records.filter(r=>r.employee_id===employeeId||(employeeId!==null&&r.employee_id===null)).sort((a,b)=>b.start.localeCompare(a.start));
 const available=(month:string)=>projects.filter(project=>!rows.some(r=>r.project===project&&effective(r,month)));
 const blank=():ContributionConfiguration=>({id:'',employee_id:employeeId,project:initialProject??available(period)[0]??'',start:period,end:null,enabled:true,base:null,employee_percent:null,employer_percent:null});
 const existing=initialProject?records.find(r=>r.employee_id===employeeId&&r.project===initialProject&&effective(r)):undefined;
 const [form,setForm]=useState<ContributionConfiguration>(()=>existing??blank());
 const fallback=employeeId?records.find(r=>r.employee_id===null&&r.project===form.project&&effective(r,form.start)):undefined;
 // Keep editable text separate from inherited values so deletion stays visible.
 const values=(row:ContributionConfiguration,defaults?:ContributionConfiguration)=>Object.fromEntries((['base','employee_percent','employer_percent'] as const).map(key=>[key,row[key]??defaults?.[key]??''])) as Record<'base'|'employee_percent'|'employer_percent',string>;
 const initial=existing??blank();
 const initialDefault=employeeId?records.find(r=>r.employee_id===null&&r.project===initial.project&&effective(r,initial.start)):undefined;
 const [draft,setDraft]=useState(()=>values(initial,employeeEdit?initialDefault:undefined));
 const [error,setError]=useState('');const [busy,setBusy]=useState(false);const [saved,setSaved]=useState('');const [pendingDelete,setPendingDelete]=useState<ContributionConfiguration|null>(null);
 const reset=()=>{const next=blank();setForm(next);setDraft(values(next));setError('');setPendingDelete(null)};
 const edit=(row:ContributionConfiguration)=>{setForm(row);setDraft(values(row));setSaved('');setError('');setPendingDelete(null)};
 const save=async()=>{setBusy(true);setError('');setSaved('');try{
   if(employeeAdd&&(form.id||!available(form.start).includes(form.project)))throw new Error('该月份的项目已配置，请关闭弹框后使用该项目行的编辑按钮。');
   await api.saveContributionConfiguration({...form,employee_id:employeeId});onSaved();
   if(employeeEdit){onClose();return;}
   const next=blank();next.project=available(period).find(project=>project!==form.project)??'';setForm(next);setDraft(values(next));setSaved('配置已保存；受影响月份的计算结果需要重新计算。');
 }catch(e){setError(String(e))}finally{setBusy(false)}};
 const remove=async()=>{if(!pendingDelete)return;setBusy(true);setError('');try{await api.deleteContributionConfiguration(pendingDelete.id);onSaved();if(employeeEdit){onClose();return;}const next=blank();next.project=projects.find(project=>!rows.some(row=>row.id!==pendingDelete.id&&row.project===project&&effective(row,period)))??'';setForm(next);setDraft(values(next));setPendingDelete(null);setSaved('配置已删除；受影响月份的结果需要重新计算。')}catch(e){setError(String(e))}finally{setBusy(false)}};
 const actions=!employeeAdd||rows.some(row=>row.employee_id===employeeId);
 const choices=!employeeEdit&&!form.id?available(form.start):projects;
 const canAdd=employeeEdit||!!form.id||choices.length>0;
 return <TaskDialog title={employeeEdit?`${initialProject} · 员工自定义方案`:employeeAdd?'新增社保公积金':form.id?`${form.project} · 主体默认配置`:'主体设置 · 社保公积金默认配置'} onClose={()=>{if(!busy)onClose()}}>
  <p className="prototype-helper">{employeeAdd?'已配置项目仅供查看；此处只新增当前月份尚未设置的缴费项目，修改已有项目请使用员工行中的编辑按钮。':employeeEdit?'仅编辑当前员工的本项目方案，不影响主体默认或其他员工。清空字段后保存可恢复跟随主体默认；填写 0% 表示采用零比例。':'本入口修改主体默认方案，影响跟随默认的员工。'}</p>
  {!employeeEdit&&!employeeAdd&&<div className="panel-head contribution-actions"><span/><button className="primary" disabled={busy} onClick={reset}>新增配置</button></div>}
  {!employeeEdit&&!form.id&&<div className="table-wrap contribution-config-table"><table><thead><tr><th>项目</th><th>有效期间</th><th>基数</th><th>员工比例</th><th>单位比例</th><th>来源</th><th>状态</th>{actions&&<th>操作</th>}</tr></thead><tbody>{rows.map(row=><tr key={row.id}><td>{row.project}</td><td>{row.start} 至 {row.end??'长期'}</td><td>{row.base??'跟随默认 / 未配置'}</td><td>{row.employee_percent===null?'跟随默认 / 未配置':`${row.employee_percent}%`}</td><td>{row.employer_percent===null?'跟随默认 / 未配置':`${row.employer_percent}%`}</td><td>{row.employee_id===null?'主体默认':row.id.startsWith('import:')?'员工导入':'员工自定义'}</td><td>{!row.enabled?'停用':row.start>period?'尚未生效':row.end&&row.end<period?'已结束':'当前生效'}</td>{actions&&<td><span className="contribution-row-actions">{!employeeAdd&&<button className="primary" disabled={busy} onClick={()=>edit(row)}>编辑</button>}{(!employeeAdd||row.employee_id===employeeId)&&<button className="danger contribution-delete" disabled={busy} onClick={()=>setPendingDelete(row)}>删除</button>}</span></td>}</tr>)}</tbody></table></div>}
  {!canAdd?<p className="prototype-helper">{employeeAdd?'当前月份的缴费项目均已设置；如需调整，请关闭后点击对应项目的“编辑”。':'暂无未配置项目；可点击表格中的编辑按钮修改已有方案。'}</p>:<div className="form-grid">
   {employeeEdit||!!form.id?<label>缴费项目<input value={form.project} readOnly/></label>:<label>缴费项目<ComboSelect value={form.project} disabled={!!form.id} onChange={e=>{setForm({...blank(),start:form.start,project:e.target.value});setDraft(values(blank()))}}>{choices.map(project=><option key={project}>{project}</option>)}</ComboSelect></label>}
   <label>生效月份<DateField type="month" value={form.start} onChange={e=>setForm({...form,start:e.target.value})}/></label><label>截止月份<DateField type="month" value={form.end??''} onChange={e=>setForm({...form,end:e.target.value||null})}/></label>
   {(['base','employee_percent','employer_percent'] as const).map((key,i)=><label key={key}>{['缴费基数','员工比例（%）','单位比例（%）'][i]}<input type="number" min="0" step="0.01" max={key==='base'?undefined:'100'} value={draft[key]} placeholder={employeeId?'留空跟随主体默认':'未配置'} onChange={e=>{setDraft({...draft,[key]:e.target.value});setForm({...form,[key]:e.target.value===''?null:e.target.value})}}/>{employeeId&&<small>{form[key]===null?`跟随主体默认${fallback?.[key]!=null?`：${fallback[key]}${key==='base'?'':'%'}`:'（未配置）'}`:'员工单独设置'}</small>}</label>)}
   {!employeeId&&<label>项目状态<ComboSelect value={String(form.enabled)} onChange={e=>setForm({...form,enabled:e.target.value==='true'})}><option value="true">启用</option><option value="false">停用</option></ComboSelect></label>}
  </div>}
  {pendingDelete&&<div className="form-error" role="alert"><p>确认删除“{pendingDelete.project}”（{pendingDelete.start} 至 {pendingDelete.end??'长期'}）？{pendingDelete.employee_id===null?'将影响所有跟随该主体默认值的员工。':'删除员工自定义方案后，未覆盖字段将回退到主体默认。'}历史计算快照保留。</p><div className="dialog-actions"><button onClick={()=>setPendingDelete(null)} disabled={busy}>取消</button><button className="danger contribution-delete" onClick={()=>void remove()} disabled={busy}>确认删除</button></div></div>}
  {error&&<div className="form-error" role="alert">{error}</div>}{saved&&<div className="validation-ok" role="status">{saved}</div>}
  <div className="dialog-actions">{form.id&&<button className="danger contribution-delete" onClick={()=>setPendingDelete(form)} disabled={busy}>删除当前配置</button>}<button onClick={onClose} disabled={busy}>关闭</button><button className="primary" onClick={()=>void save()} disabled={busy||!canAdd||!form.project}>{busy?'保存中…':employeeEdit||!!form.id?'保存修改':'保存配置'}</button></div>
 </TaskDialog>;
}
