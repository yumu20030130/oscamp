#!/bin/sh

#make run A=labs/lab1 > /tmp/lab1_output.log
make run A=labs/lab1 2>/dev/null | tail -n -4
