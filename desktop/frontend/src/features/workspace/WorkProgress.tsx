import { useEffect,useState } from 'react';
import { api } from '../../api';
import type { Section,WorkProgress } from '../../types';
export function useWorkProgress(scope:{period:string;payout_month:string;region:string},refresh=0){
 const [data,setData]=useState<WorkProgress|null>(null);const [loading,setLoading]=useState(true);const [error,setError]=useState('');
 useEffect(()=>{let active=true;setLoading(true);setData(null);setError('');const timer=setTimeout(()=>{void api.workProgress(scope).then(value=>{if(active)setData(value)}).catch(reason=>{if(active)setError(String(reason))}).finally(()=>{if(active)setLoading(false)});},180);return()=>{active=false;clearTimeout(timer)}},[scope.period,scope.payout_month,scope.region,refresh]);
 return {data,loading,error};
}
export function WorkProgressSteps({data,loading,onNavigate}:{data:WorkProgress|null;loading:boolean;onNavigate?:(section:Section)=>void}){
 if(loading)return <div className="import-progress" role="status">正在核对本期进度…</div>;
 if(!data)return null;
 const current=data.steps.findIndex(step=>step.state!=='done');
 return <div className="workflow-flow progress-flow" aria-label="本期实际工作进度">{data.steps.map((step,index)=><span className="progress-step-slot" key={step.label}>{index>0&&<b aria-hidden="true">›</b>}<button className={`${step.state} ${index===current?'current':''}`} title={step.detail} onClick={()=>onNavigate?.(step.target)} disabled={!onNavigate} aria-label={`${step.label}：${step.detail}`}><i aria-hidden="true">{step.state==='done'?'✓':step.state==='warning'?'!':String(index+1).padStart(2,'0')}</i>{step.label}</button></span>)}</div>;
}
