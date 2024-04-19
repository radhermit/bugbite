require 'asciidoctor'
require 'asciidoctor/extensions'

module Bugbite
  module Documentation
    class LinkCmdProcessor < Asciidoctor::Extensions::InlineMacroProcessor
      def process(parent, target, attrs)
        if parent.document.backend == 'manpage'
          "#{target}(#{attrs[1]})"
        elsif parent.document.backend == 'html5'
          %(<a href="#{target}.html">#{target}(#{attrs[1]})</a>)
        end
      end
    end
  end
end

Asciidoctor::Extensions.register do
  inline_macro Bugbite::Documentation::LinkCmdProcessor, :linkcmd
end
