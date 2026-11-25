// One-time calculation for the chip piece: expected by beta, deviations, expected by valuation, correlations.
const fs=require('fs');const S='/home/user/fa-data-split/';
function csv(f){const L=fs.readFileSync(f,'utf8').trim().split('\n');const h=L[0].split(',');return L.slice(1).map(l=>{const c=l.split(',');const o={};h.forEach((k,i)=>o[k]=c[i]);return o;});}
function weekly(t){const p=JSON.parse(fs.readFileSync(S+t+'/prices.json'));const m=new Map();
  for(const r of p){const d=new Date(r.date+'T00:00:00Z');const day=(d.getUTCDay()+6)%7;m.set(new Date(d-day*864e5).toISOString().slice(0,10),r.close);}return m;}
function rets(t){const w=weekly(t);const k=[...w.keys()].sort();const o=new Map();for(let i=1;i<k.length;i++)o.set(k[i],w.get(k[i])/w.get(k[i-1])-1);return o;}
function sd(a){const m=a.reduce((x,y)=>x+y,0)/a.length;return Math.sqrt(a.reduce((x,y)=>x+(y-m)**2,0)/(a.length-1));}
function corr(x,y){const n=x.length,mx=x.reduce((a,b)=>a+b)/n,my=y.reduce((a,b)=>a+b)/n;let sxy=0,sx=0,sy=0;for(let i=0;i<n;i++){sxy+=(x[i]-mx)*(y[i]-my);sx+=(x[i]-mx)**2;sy+=(y[i]-my)**2;}return sxy/Math.sqrt(sx*sy);}
const all=csv('sp500.csv');const chips=all.filter(r=>r.industry.startsWith('3674'));
const spy=rets('SPY');const W='2026-11-16';const keys=[...spy.keys()].sort().filter(k=>k<W).slice(-104);const EW=-6.563;
const f=(x,d=1)=>x===''||x===undefined||isNaN(x)?'':(+x).toFixed(d);
console.log('ticker change beta expBeta diffPts dev n | sp500run: time+rates common company expVal | g10_1 g10_2 flags');
const rows=[];
for(const r of chips){const t=r.ticker,b=+r.beta1,ch=+r.change_pct;let dev='',n=0,expB='';
  if(r.beta1!==''){const rt=rets(t);const res=keys.filter(k=>rt.has(k)).map(k=>rt.get(k)-b*spy.get(k));n=res.length;expB=b*EW;dev=(ch-expB)/(sd(res)*100);}
  const tr=r.g_rates_pct===''?'':(+r.g_time_pct)+(+r.g_rates_pct)+(+r.g_facts_pct);const com=r.g_growth_common_pct,co=r.g_growth_company_pct;const expV=tr===''?'':tr+(+com);
  rows.push({t,ch,b,expB,dev,expV,co:co===''?NaN:+co,g1:+r.growth10_1*100,stale:r.flags.includes('stale')});
  console.log(t,f(ch),f(b,2),f(expB),f(expB===''?'':ch-expB),f(dev),n,'|',f(tr),f(com),f(co),f(expV),'|',f(r.growth10_1*100),f(r.growth10_2*100),r.flags);}
const ok=rows.filter(r=>!r.stale);const m=a=>a.reduce((x,y)=>x+y,0)/a.length;
console.log('mean change (15, no SWKS)',m(ok.map(r=>r.ch)).toFixed(2),'mean expBeta',m(ok.filter(r=>r.expB!=='').map(r=>r.expB)).toFixed(2),'mean change of those',m(ok.filter(r=>r.expB!=='').map(r=>r.ch)).toFixed(2));
const v=ok.filter(r=>r.expV!=='');console.log('valuation set n',v.length,'mean change',m(v.map(r=>r.ch)).toFixed(2),'mean expVal',m(v.map(r=>r.expV)).toFixed(2));
const g=ok.filter(r=>!isNaN(r.g1)&&r.t!=='AMD');console.log('corr change vs needed growth (n',g.length,')',corr(g.map(r=>r.g1),g.map(r=>r.ch)).toFixed(2));
const bb=ok.filter(r=>r.expB!=='');console.log('corr change vs beta (n',bb.length,')',corr(bb.map(r=>r.b),bb.map(r=>r.ch)).toFixed(2));
console.log('devs beyond -1:',bb.filter(r=>r.dev<=-1).map(r=>r.t).join(' '),'| within:',bb.filter(r=>Math.abs(r.dev)<1).map(r=>r.t).join(' '));
// S&P 500 without chips: same test count
const oth=all.filter(r=>!r.industry.startsWith('3674')&&r.change_pct!==''&&!r.flags.includes('stale'));console.log('others n',oth.length,'mean change',m(oth.map(r=>+r.change_pct)).toFixed(2));
