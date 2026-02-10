
(debug "macros.scm")

(define cadr (lambda (l) (car (cdr l))))
(define cddr (lambda (l) (cdr (cdr l))))

(define map
    (lambda (fn list) 
        (if (null? list) 
            ()
            (cons (fn (car list)) (map fn (cdr list)))))
)
(debug "1. macros.scm")
 
(define-syntax let
    (lambda (bindings . body)
        (debug bindings)
        `((lambda ,(map car bindings) ,@body) ,@(map cadr bindings)))
)

(define-syntax begin (lambda exprs `((lambda () ,@exprs))))

(define-syntax unless 
    (lambda (cond . body) `(if (not ,cond) (begin ,@body)))
)

(debug "loaded.")