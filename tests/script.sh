#!/usr/bin/env bash

# script takes one argument, which is what type of explanations you're running
#
# for example "./script.sh naive" 

mkdir -p output

# runs the test, outputs to "output/type/file"
# input 1: file
# input 2: mzn
# input 3: type
run(){
  echo
  echo "----------------------------------------------"
  echo "---------------- Starting $1 -----------------"
  echo "----------------------------------------------"

  mkdir -p "output/$3"
  OUTPUT=$(echo "$1" | cut -d "/" -f 2)

  echo "minizinc --time-limit 600000 -a -s -o \"output/$3/${OUTPUT}\" --output-time --solver Pumpkin \"$1\" \"$2\""
  echo
  
  minizinc --time-limit 600000 -a -s -o "output/$3/${OUTPUT}" --output-time --solver Pumpkin "$1" "$2"
  }

# scans directory for data files and runs it on the input mzn
#
# mzn should be in the folder, but if you input "../blah.mzn" it will still run (hopefully)
# input 1: folder
# input 2: mzn
# input 3: type
scan(){
  for file in "$1"/*; do
    if [ "$file" == "$1/$2.mzn" ]; then
      continue
    fi

    run "$file" "$1/$2.mzn" "$3"
  done
}

# example running a dataset:
# it basically scans folder "la", which has "jobshop.mzn" in it and outputs the data in "output/$1"
# scan "la" "jobshop" "$1"

# scan "orb" "jobshop" "$1"

scan "ta" "jobshop" "$1"

# scan "jobshop" "jobshop2" "$1"

