DESTDIR ?= build
all: html man
.PHONY: all html man

html:
	asciidoctor -b html5 $(wildcard *.adoc) -D "${DESTDIR}/html"

man:
	asciidoctor -b manpage $(wildcard *.adoc) -D "${DESTDIR}/man"
