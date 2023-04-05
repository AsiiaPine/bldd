Backward ldd
=======

How does it work
_______

All ELF files contains data about the libraries used by the program, therefore such as ***ldd*** bash commands exists, which parse an ELF dynamic table to find dependencies.  
The bldd works with directories and collect the **ldd** command output to make a **HashMap** of all lib names and files used a lib.

_____
The ouput looks like that:
````
```
Lib name        libdl.so.2 => /usr/lib/libdl.so.2 (0x00007f3bc7a34000)
______________________________n_exec(1)
x86_64__________ 
elf-Linux-x64-bash
```
````



