/* note these headers are all provided by newlib - you don't need to provide them */
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/fcntl.h>
#include <sys/times.h>
#include <sys/errno.h>
#include <sys/time.h>
#include <stdio.h>

#include <sys/syscalls.h>

/* pointer to array of char * strings that define the current environment variables */
char *__env[1] = { 0 };
char **environ = __env;

//1
void _exit()	
{	//enter an infinite loop (it's close to exiting...)
	while (1);
}

//2
int close(int file)
{
	errno = ENOTSUP;
	return -1;
}

//3
int execve(char *name, char **argv, char **env)
{
	errno = ENOMEM;
	return -1;
}

//4
int fork()
{
	errno = EAGAIN;
	return -1;
}

//5
int fstat(int file, struct stat *st)
{
	st->st_mode = S_IFCHR;
	return 0;
}

//6
int getpid()
{
	return 1;
}

//7
int isatty(int file)
{
	return 1;
}

//8
int kill(int pid, int sig)
{
	errno = EINVAL;
	return -1;
}

//9
int link(char *old, char *new)
{
	errno = EMLINK;
	return -1;
}

//10
int lseek(int file, int ptr, int dir)
{
	return 0;
}

//11
int open(const char *name, int flags, ...)
{
	return -1;
}

//12
int read(int file, char *ptr, int len)
{
	return 0;
}

//13
caddr_t sbrk(int incr)
{	//doesnt do anything anyways
	asm("mov (SYS_sbrk),%%eax\n\t"
		"mov $13,%%ebx\n\t"
		"int $0x30"
		:
		:
		: "%eax", "%ebx");
	
	errno = ENOMEM;
	return (caddr_t) -1;
}

//14
int stat(const char *file, struct stat *st)
{
	st->st_mode = S_IFCHR;
	return 0;
}

//15
clock_t times(struct tms *buf)
{
	return -1;
}

//16
int unlink(char *name)
{
	errno = ENOENT;
	return -1;
}

//17
int wait(int *status)
{
	errno = ECHILD;
	return -1;
}

//18
int write(int file, char *ptr, int len)
{
	errno = ENOTSUP;	//not supported (yet)
	return -1;
}

//19
int gettimeofday(struct timeval *p, struct timezone *z);

