; PROVENANCE: Copied from
;   https://github.com/paul-gauthier/aider (main branch)
;   aider/queries/tree-sitter-language-pack/c-tags.scm
; License: MIT (tree-sitter-c, MIT License)
; Bundled via: tree-sitter-language-pack (MIT License)
; See crates/bts-map/NOTICE for per-grammar attribution.
;
; DIVERGENCE: The vendored copy in vendor/queries/c-tags.scm is the verbatim
; upstream file. This copy adds a call_expression reference capture absent in
; aider's original .scm (aider backfills refs via pygments for C/C++). The
; addition is needed so that per-language non-empty ref tests pass for C.
; See vendor/queries/c-tags.scm for the unmodified original.

(struct_specifier name: (type_identifier) @name.definition.class body:(_)) @definition.class

(declaration type: (union_specifier name: (type_identifier) @name.definition.class)) @definition.class

(function_declarator declarator: (identifier) @name.definition.function) @definition.function

(type_definition declarator: (type_identifier) @name.definition.type) @definition.type

(enum_specifier name: (type_identifier) @name.definition.type) @definition.type

; Added: call site references (not in original aider scm; added for bts-map ref tracking)
(call_expression function: (identifier) @name.reference.call) @reference.call
