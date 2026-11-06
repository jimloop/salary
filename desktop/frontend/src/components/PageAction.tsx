import { createPortal } from 'react-dom';
import type { ReactNode } from 'react';
export function PageAction({children}:{children:ReactNode}) {
  const target=typeof document==='undefined'?null:document.getElementById('page-actions');
  return target?createPortal(children,target):null;
}
