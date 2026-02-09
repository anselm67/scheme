
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
 
(define let-expr 
    (lambda (keyword bindings . body)
        (debug bindings)
        `((lambda ,(map car bindings) ,@body) ,@(map cadr bindings)))
)


(define v '((x 1) (y 2)))
(define p '((debug x) (debug y) (+ x y)))

(define f (lambda (x) `(debug ,x)))

(debug "loaded.")