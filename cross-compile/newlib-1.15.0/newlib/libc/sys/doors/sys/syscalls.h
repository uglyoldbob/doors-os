//This file defines the function numbers for the system calls.
//this must match what is found in entrance.asm (line 392)
#define	SYS_close	1
#define SYS_execve	2
#define	SYS_exit	3
#define SYS_fork	4
#define SYS_fstat	5
#define	SYS_getpid	6
#define SYS_isatty	7
#define SYS_kill	8
#define	SYS_link	9
#define	SYS_lseek	10
#define	SYS_open	11
#define	SYS_read	12
#define SYS_sbrk	13
#define SYS_stat	14
#define SYS_time	15
#define SYS_times	16
#define	SYS_unlink	17
#define SYS_wait	18
#define	SYS_write	19

#define SYS_MAX		20
