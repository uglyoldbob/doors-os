#                                               -*- Autoconf -*-
# Process this file with autoconf to produce a configure script.

AC_PREREQ(2.61)
AC_INIT(doors-os, 0.20, thomas.epperson@gmail.com)

AC_CONFIG_SRCDIR([src/file.h])
AC_CONFIG_HEADER([config.h])

AC_ARG_ENABLE(	[debug],
				[  --enable-debug    Turn on debugging],
				[case "${enableval}" in
					yes) 	debug=true ;;
					no)  	debug=false ;;
					*) 		AC_MSG_ERROR([bad value ${enableval} for --enable-debug]) ;;
				esac],
				[debug=false])
AM_CONDITIONAL([DEBUG], [test x$debug = xtrue])



AC_ARG_ENABLE(	[bootimage],
				[  --enable-bootimage    Configure boot disk type],
				[case "${enableval}" in
					floppy) 	bootimage=floppy ;;
					cd)  		bootimage=cd ;;
					*) 			AC_MSG_ERROR([bad value ${enableval} for --enable-bootimage]) ;;
				esac],
				[bootimage=false])
AM_CONDITIONAL([BOOTIMAGE_FLOPPY], [test x$bootimage = xfloppy])
AM_CONDITIONAL([BOOTIMAGE_CD], [test x$bootimage = xcd])


case $host in
	i*86*-*-*) KERNEL_SYSTEM='x86/dma.$(OBJEXT)				\
							  x86/entrance.$(OBJEXT) 		\
							  x86/gdt.$(OBJEXT)				\
							  x86/floppy.$(OBJEXT) 			\
							  x86/gdb-support.$(OBJEXT) 	\
							  x86/gdb-stub.$(OBJEXT)		\
							  x86/interrupt_table.$(OBJEXT)	\
							  x86/keyboard.$(OBJEXT)		\
							  x86/main.$(OBJEXT)			\
							  x86/memory.$(OBJEXT)			\
							  x86/PIC.$(OBJEXT)				\
							  x86/serial.$(OBJEXT)			\
							  x86/spinlock.$(OBJEXT)		\
							  x86/tss.$(OBJEXT)				\
							  x86/video.$(OBJEXT)			\
							  x86/vmm.$(OBJEXT)				\
							  x86/debug.$(OBJEXT)'
				cp x86/link.ld link.ld
				set_X86=true
				AC_MSG_NOTICE($KERNEL_SYSTEM)
				AC_MSG_NOTICE($KERNEL_LDFLAGS) ;;
	*)       	AC_MSG_ERROR([bad value ${host} for host]) ;;
esac
AM_CONDITIONAL([X86], [${set_X86}])
AC_SUBST([KERNEL_SYSTEM])


AM_INIT_AUTOMAKE([-Wall -Werror foreign])

# Checks for programs.
AC_PROG_CXX
AC_PROG_CC
AM_PROG_AS

# Checks for libraries.

# Checks for header files.

# Checks for typedefs, structures, and compiler characteristics.
AC_C_CONST
AC_TYPE_SIZE_T
AC_C_VOLATILE

AC_CONFIG_FILES([Makefile
                 src/Makefile
				 loader/Makefile])
AC_OUTPUT
