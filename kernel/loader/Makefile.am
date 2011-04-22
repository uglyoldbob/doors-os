AUTOMAKE_OPTIONS = subdir-objects
AM_CFLAGS = -nostdlib -fno-builtin -fno-exceptions -gdwarf-2
AM_CXXFLAGS = -nostdlib -fno-builtin -fno-rtti -fno-exceptions -gdwarf-2

AM_LDFLAGS = -T link.ld

bin_PROGRAMS = loader

loader_SOURCES = loadera.s loaderb.c
loader_LDADD = $(LIBOBJS)
