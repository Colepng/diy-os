// .intel_syntax noprefix  
//
// .globl into_usermode
// .globl usermode
//
// into_usermode:
//     ret
//     mov ax, (4 * 8) | 3
//     mov ds, ax
//     mov es, ax
//     mov fs, ax
//     mov gs, ax
//
//     // stack frame setup
//     mov eax, esp
//     push (4 * 8) | 3 // data selector
//     push rax // current esp
//     pushf // eflags
//     push (3 * 8) | 3 // code selector (ring 3 code with bottom 2 bits set for ring 3)
//     push usermode // instruction address to return to
//     iret
//
//usermode:
//     cli

; .global into_usermode
; .extern usermode
; into_usermode:
;     cli
; //enable system call extensions that enables sysret and syscall
; 	mov rcx, 0xc0000082
; 	wrmsr
; 	mov rcx, 0xc0000080
; 	rdmsr
; 	or eax, 1
; 	wrmsr
; 	mov rcx, 0xc0000081
; 	rdmsr
; 	mov edx, 0x00180008
; 	wrmsr
; 
;     // call usermode
; 	mov ecx, usermode // to be loaded into RIP
; 	mov r11, 0x202 // to be loaded into EFLAGS
	sysretq
