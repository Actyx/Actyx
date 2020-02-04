#!/bin/bash
# debug tool to watch the status of a sqlite ipfs index store
if [ -z "$1" ]; then
  DB="pond0.sqlite"
else
  echo $1
  DB="$1"
fi

echo "roots"
sqlite3 $DB 'select * from roots'

echo
echo "recvlog"
sqlite3 $DB 'select source,count(*) from recvlog group by source'

echo
echo "sendlog"
sqlite3 $DB 'select count(*) from sendlog'

echo
echo "meta"
sqlite3 $DB 'select * from meta'
