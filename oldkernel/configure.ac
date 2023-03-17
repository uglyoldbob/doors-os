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
  i*86*-*-* )
	BUILD_FOR="X86"
	AC_DEFINE(X86, [1], [Enable code specific to x86 architecture])
	AC_MSG_NOTICE("Building for x86 architecture")
	;;
  * )
	BUILD_FOR="BROKEN"
	AC_MSG_ERROR("Possibly invalid build target $host_os")
	;;
esac
AC_SUBST(BUILD_FOR)
AM_CONDITIONAL([X86], [test \"$BUILD_FOR\" = \"X86\"])


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

AC_CONFIG_FILES([Makefile])
AC_OUTPUT
