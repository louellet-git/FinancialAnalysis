# Largest absolute difference of the numeric cells of two CSV files with the same shape (text cells must be equal).
BEGIN{FS=","; max=0; bad=0}
NR==FNR{for(i=1;i<=NF;i++) a[FNR,i]=$i; n[FNR]=NF; next}
{ if(NF!=n[FNR]) bad++; for(i=1;i<=NF;i++){ x=a[FNR,i]; y=$i; if(x==y) continue;
    if(x ~ /^-?[0-9.]+(e-?[0-9]+)?$/ && y ~ /^-?[0-9.]+(e-?[0-9]+)?$/){ d=x-y; if(d<0)d=-d; if(d>max){max=d; where=FNR","i} cells++ } else { bad++; print "text cell differs: row " FNR " col " i ": " x " | " y } } }
END{printf "cells that differ: %d; largest absolute difference: %.3g (row,col %s); text or shape differences: %d\n", cells, max, where, bad}
