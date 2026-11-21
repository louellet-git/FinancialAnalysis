// One-time check: weekly rest deviations (104 weeks before the week to 2026-11-20).
const fs=require('fs');const S='/home/user/fa-data-split/';
function weekly(t){const p=JSON.parse(fs.readFileSync(S+t+'/prices.json'));const m=new Map();
  for(const r of p){const d=new Date(r.date+'T00:00:00Z');const day=(d.getUTCDay()+6)%7;const mon=new Date(d-day*864e5).toISOString().slice(0,10);m.set(mon,r.close);}
  return m;}
function rets(t){const w=weekly(t);const k=[...w.keys()].sort();const o=new Map();for(let i=1;i<k.length;i++)o.set(k[i],w.get(k[i])/w.get(k[i-1])-1);return o;}
const beta={AAPL:1.0893,NVDA:1.6743,MSFT:1.0814,GOOG:1.1321,AMZN:1.2874,AVGO:1.5695,META:1.2599,TSLA:1.5976,LLY:0.6808,'BRK-B':0.7325};
const spy=rets('SPY');const W='2026-11-16';const keys=[...spy.keys()].sort().filter(k=>k<W).slice(-104);
const EW=-0.06563;
function sd(a){const m=a.reduce((x,y)=>x+y,0)/a.length;return Math.sqrt(a.reduce((x,y)=>x+(y-m)**2,0)/(a.length-1));}
for(const t of Object.keys(beta)){const r=rets(t);const res=keys.map(k=>r.get(k)-beta[t]*spy.get(k));const s=sd(res);
  const wk=r.get(W);console.log(t,'week',(wk*100).toFixed(2),'sd',(s*100).toFixed(2),'restSPY',((wk-beta[t]*spy.get(W))/s).toFixed(2),'restEW',((wk-beta[t]*EW)/s).toFixed(2),'betaLine',(beta[t]*EW*100).toFixed(1));}
const chips=['MU','AMD','AMAT','INTC','TXN','AVGO'];const cr=chips.map(rets);const nv=rets('NVDA');
const diff=k=>nv.get(k)-cr.reduce((x,r)=>x+r.get(k),0)/cr.length;
const s=sd(keys.map(diff));console.log('NVDA less 6 chips: week',(diff(W)*100).toFixed(2),'sd',(s*100).toFixed(2),'dev',(diff(W)/s).toFixed(2),'n',keys.length,keys[0],keys[keys.length-1]);
