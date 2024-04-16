DESTDIR ?= build/man
docs := $(wildcard *.adoc)
options := -b manpage
all: man
.PHONY: all man

man:
	asciidoctor $(docs) $(options) -D "${DESTDIR}"
