; PROVENANCE: Copied from
;   https://github.com/paul-gauthier/aider (main branch)
;   aider/queries/tree-sitter-language-pack/swift-tags.scm
; License: MIT (tree-sitter-swift by alex-pinkus, MIT License)
; Bundled via: tree-sitter-language-pack (MIT License)
; Grammar crate used: npezza93-tree-sitter-swift =0.4.4 (same grammar, different publisher).
; See crates/bts-map/NOTICE for per-grammar attribution.
;
; DIVERGENCE: The vendored copy in vendor/queries/swift-tags.scm is the verbatim
; upstream file. This copy adds a call_expression reference capture absent in
; aider's original .scm (aider backfills refs via pygments for Swift). The
; addition is needed so that per-language non-empty ref tests pass for Swift.
; See vendor/queries/swift-tags.scm for the unmodified original.

(class_declaration
  name: (type_identifier) @name.definition.class) @definition.class

(protocol_declaration
  name: (type_identifier) @name.definition.interface) @definition.interface

(class_declaration
    (class_body
        [
            (function_declaration
                name: (simple_identifier) @name.definition.method
            )
            (subscript_declaration
                (parameter (simple_identifier) @name.definition.method)
            )
            (init_declaration "init" @name.definition.method)
            (deinit_declaration "deinit" @name.definition.method)
        ]
    )
) @definition.method

(protocol_declaration
    (protocol_body
        [
            (protocol_function_declaration
                name: (simple_identifier) @name.definition.method
            )
            (subscript_declaration
                (parameter (simple_identifier) @name.definition.method)
            )
            (init_declaration "init" @name.definition.method)
        ]
    )
) @definition.method

(class_declaration
    (class_body
        [
            (property_declaration
                (pattern (simple_identifier) @name.definition.property)
            )
        ]
    )
) @definition.property

(property_declaration
    (pattern (simple_identifier) @name.definition.property)
) @definition.property

(function_declaration
    name: (simple_identifier) @name.definition.function) @definition.function

; Added: call site references (not in original aider scm; added for bts-map ref tracking)
(call_expression
    (simple_identifier) @name.reference.call) @reference.call
