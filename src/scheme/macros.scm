

(define! cadr (lambda (l) (car (cdr l))))
(define! cddr (lambda (l) (cdr (cdr l))))

(define-syntax define 
    (lambda (name_and_params . body) 
        (if (symbol? name_and_params)
            `(define! ,name_and_params ,(car body))
            `(define! ,(car name_and_params) (lambda ,(cdr name_and_params) ,@body)))
    )
)

(define map
    (lambda (fn list) 
        (if (null? list) 
            ()
            (cons (fn (car list)) (map fn (cdr list)))))
)
 
(define-syntax let
    (lambda (bindings . body)
        (debug bindings)
        `((lambda ,(map car bindings) ,@body) ,@(map cadr bindings)))
)

(define-syntax begin (lambda exprs `((lambda () ,@exprs))))

(define-syntax unless 
    (lambda (cond . body) `(if (not ,cond) (begin ,@body)))
)
