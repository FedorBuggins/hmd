#!/bin/bash

run() {
  i=$1;
  status="";
  for stage in "${!stages[@]}"; do
    if (( stage < i )); then
      status="$status✅ ${stages[$stage]}\n";
    fi
    if (( stage == i )); then
      status="$status🟩 ${stages[$stage]}\n";
    fi
    if (( stage > i )); then
      status="$status🟨 ${stages[$stage]}\n";
    fi
  done
  echo -e "$status`date +%FT%T`" > status.log;
}

complete() {
  i=$1;
  status="";
  for stage in "${!stages[@]}"; do
    if (( stage <= i )); then
      status="$status✅ ${stages[$stage]}\n";
    fi
    if (( stage > i )); then
      status="$status🟨 ${stages[$stage]}\n";
    fi
  done
  echo -e "$status`date +%FT%T`" > status.log;
}

panic() {
  i=$1;
  status="";
  for stage in "${!stages[@]}"; do
    if (( stage < i )); then
      status="$status✅ ${stages[$stage]}\n";
    fi
    if (( stage == i )); then
      status="$status❌ ${stages[$stage]}\n";
    fi
    if (( stage > i )); then
      status="$status🟥 ${stages[$stage]}\n";
    fi
  done
  echo -e "$status`date +%FT%T`" > status.log;
  exit 1;
}
