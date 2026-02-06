
(define let-expr 
    (lambda (bindings . body)
        `((lamba ,(map car ,@bindings) ,@ body) ,@(map cadr bindings))
    )
)
