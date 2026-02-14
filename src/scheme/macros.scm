

(define! cadr (lambda (l) (car (cdr l))))
(define! cddr (lambda (l) (cdr (cdr l))))

(define-syntax define 
    (lambda (name_and_params . body) 
        (if (symbol? name_and_params)
            `(define! ,name_and_params ,(car body))
            `(define! ,(car name_and_params) (lambda ,(cdr name_and_params) ,@body)))
    )
)

(define (map fn list) 
    (if (null? list) 
        ()
        (cons (fn (car list)) (map fn (cdr list))))
)

 
(define named_let_
    (lambda (name bindings body) 
        `(letrec ((,name (lambda ,(map car bindings) ,@body))) 
            (,name ,@(map cadr bindings)) ) )
)

(define regular_let_
    (lambda (bindings body)
            `((lambda ,(map car bindings) ,@body) ,@(map cadr bindings)))
)

(define-syntax let 
    (lambda args 
        (if (symbol? (car args))
            (named_let_ (car args) (cadr args) (cddr args))
            (regular_let_ (car args) (cdr args))))
)

(define-syntax letrec
    (lambda (bindings . body)
        ; TODO Once map is fixed, we can use:
        ; (let ((vars (map car bindings)) (vals (cadr bindings)) 
        ;     ...
        ;     ,@(map (lambda (var val) (list 'set! var val)) vars vals)))
        `(let ,(map (lambda (varval) (list (car varval) ''*undefined*)) bindings) 
            ,@(map (lambda (varval) (list 'set! (car varval) (cadr varval))) bindings)
            ,@body)
))



(define-syntax begin (lambda exprs `((lambda () ,@exprs))))

(define-syntax unless 
    (lambda (cond . body) `(if ,cond () (begin ,@body)))
)

(define-syntax or 
    (lambda exprs
        (if (null? exprs)
            #f
            `(let ((first ,(car exprs)))
                (if first first (or ,@(cdr exprs))))
        )
    )
)

(define (assert-equal object value)
    (if (equal? object value) #t (error "test failed."))
)

