# スクリプトの終わりに、& で起動したジョブが終わるのを待つ
function late () { sleep 1; echo late; }
late &
echo first
