DESTDIR ?= build
all: html man
.PHONY: all html man

html:
	asciidoctor -I . -r asciidoctor -b html5 $(wildcard *.adoc) -D "${DESTDIR}/html"

man:
	asciidoctor -I . -r asciidoctor -b manpage $(wildcard bite*.adoc) -D "${DESTDIR}/man"
