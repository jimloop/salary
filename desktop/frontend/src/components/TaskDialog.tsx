import { useEffect } from 'react';
import type { ReactNode } from 'react';
export function TaskDialog({title,onClose,children}:{title:string;onClose:()=>void;children:ReactNode}) {
  useEffect(()=>{const handler=(event:KeyboardEvent)=>{if(event.key==='Escape')onClose()};document.addEventListener('keydown',handler);return()=>document.removeEventListener('keydown',handler)},[onClose]);
  return <div className="modal-backdrop" onMouseDown={onClose}><div className="dialog task-dialog" role="dialog" aria-modal="true" aria-label={title} onMouseDown={event=>event.stopPropagation()}><div className="dialog-head"><h2>{title}</h2><button aria-label="关闭" onClick={onClose}>×</button></div>{children}</div></div>;
}
