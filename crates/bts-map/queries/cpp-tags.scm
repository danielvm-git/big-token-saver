; PROVENANCE: Copied from
;   https://github.com/paul-gauthier/aider (main branch)
;   aider/queries/tree-sitter-language-pack/cpp-tags.scm
; License: MIT (tree-sitter-cpp, MIT License)
; Bundled via: tree-sitter-language-pack (MIT License)
; See crates/bts-map/NOTICE for per-grammar attribution.
;
; DIVERGENCE: The vendored copy in vendor/queries/cpp-tags.scm is the verbatim
; upstream file. This copy adds a call_expression reference capture absent in
; aider's original .scm (aider backfills refs via pygments for C/C++). The
; addition is needed so that per-language non-empty ref tests pass for C++.
; See vendor/queries/cpp-tags.scm for the unmodified original.

(struct_specifier name: (type_identifier) @name.definition.class body:(_)) @definition.class

(declaration type: (union_specifier name: (type_identifier) @name.definition.class)) @definition.class

(function_declarator declarator: (identifier) @name.definition.function) @definition.function

(function_declarator declarator: (field_identifier) @name.definition.function) @definition.function

(function_declarator declarator: (qualified_identifier scope: (namespace_identifier) @local.scope name: (identifier) @name.definition.method)) @definition.method

(type_definition declarator: (type_identifier) @name.definition.type) @definition.type

(enum_specifier name: (type_identifier) @name.definition.type) @definition.type

(class_specifier name: (type_identifier) @name.definition.class) @definition.class

; Added: call site references (not in original aider scm; added for bts-map ref tracking)
(call_expression function: (identifier) @name.reference.call) @reference.call
