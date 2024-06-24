#!/bin/bash

nasm -f bin -o ~/Documents/projects/diy-os/bin/hello_world ~/Documents/projects/diy-os/bin/hello_world.asm
tar cfv ~/Documents/projects/diy-os/bin/hello_world.tar ~/Documents/projects/diy-os/bin/hello_world
