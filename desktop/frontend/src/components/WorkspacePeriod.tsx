import { createContext,useContext,useState } from 'react';
import type { ReactNode } from 'react';
type PeriodValue={period:string;payoutMonth:string;region:string;setPeriod:(value:string)=>void;setPayoutMonth:(value:string)=>void;setRegion:(value:string)=>void};
const PeriodContext=createContext<PeriodValue|null>(null);
function useLocalPeriod(regionValue=''):PeriodValue{const [period,setPeriod]=useState('2026-10');const [payoutMonth,setPayoutMonth]=useState('2026-11');const [region,setRegion]=useState(regionValue);return {period,payoutMonth,region,setPeriod,setPayoutMonth,setRegion}}
export function WorkspacePeriodProvider({children,region=''}:{children:ReactNode;region?:string}){const value=useLocalPeriod(region);return <PeriodContext.Provider value={value}>{children}</PeriodContext.Provider>}
export function useWorkspacePeriod(region=''):PeriodValue{const context=useContext(PeriodContext);const local=useLocalPeriod(region);return context??local}
