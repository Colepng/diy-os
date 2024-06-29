[bits 64]

    org 0x08048000

    ; ELF header
ehdr:
    db 0x7f, 'E', 'L', 'F'
    ; "Class" = 2, 64-bit
    db 2
    ; Endianness = 1, little
    db 1
    ; ELF version = 1
    db 1
    ; OS ABI, unused, should be 0
    db 0
    ; Extended ABI byte + 7 bytes padding. Leave as 0, it's ignored
    dq 0
    ; ELF file type. 2 = executable
    dw 2
    ; arch x86_64
    dw 0x3e
    ; elf version
    dd 1
    ; entry
    dq _start
    ; program header offset
    dq phdr - $$
    ; section header offset
    dq 0
    ;flags
    dd 0
    ; header size
    dw ehdrsize

    ; program header size
    dw phdrsize
    ; number of program header
    dw 1

    dw 0
    dw 0
    dw 0 

    ehdrsize    equ     $ - ehdr


phdr:
    ; type = loadable
    dd 1
    ; Program header flags. 5 = Not writable. (bits 0, 1, and 2 = executable,
    ; writable, and readable, respectively)
    dd 5
    ; offset
    dq 0
    ; The VA to place the segment at.
    dq $$
    ; The "phyiscal address". Don't think it's used, set to same as VA.
    dq $$
    ; file size
    dq filesize
    ; mem size
    dq filesize
    dq 0x1000

    phdrsize    equ     $ - phdr

_start:
    ; mov rax, 2
    ; mov rdx, 35
    ; mov rsi, 30
    ; int 0x80
    push 70
    mov rax, 1
    mov rsi, rsp
    mov rdx, 1
    int 0x80
    mov rax, 0
    int 0x80


    filesize    equ     $ - $$
