DESTDIR ?= build
OPTIONS := -I . -r adoc_exts --failure-level WARNING
all: html man
.PHONY: all html man

html:
	asciidoctor ${OPTIONS} -b html5 -D "${DESTDIR}/html" $(wildcard *.adoc)

man:
	asciidoctor ${OPTIONS} -b manpage -D "${DESTDIR}/man" $(wildcard bite*.adoc)
