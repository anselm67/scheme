; macros.scm
; This file is bundled with the interpretor and defines all the well known macros
; and functions that aren't defined as primitives.

(define! cadr (lambda (l) (car (cdr l))))
(define! cddr (lambda (l) (cdr (cdr l))))
(define! caar (lambda (l) (car (car l))))
(define! cdar (lambda (l) (cdr (car l))))

(define-syntax define 
    (lambda (name_and_params . body) 
        (if (symbol? name_and_params)
            `(define! ,name_and_params ,(car body))
            `(define! ,(car name_and_params) (lambda ,(cdr name_and_params) ,@body)))
    )
)

(define (map_one_ fn list) 
    (if (null? list) 
        ()
        (cons (fn (car list)) (map_one_ fn (cdr list))))
)

(define named_let_
    (lambda (name bindings body) 
        `(letrec ((,name (lambda ,(map_one_ car bindings) ,@body))) 
            (,name ,@(map_one_ cadr bindings)) ) )
)

(define regular_let_
    (lambda (bindings body)
            `((lambda ,(map_one_ car bindings) ,@body) ,@(map_one_ cadr bindings)))
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
        `(let ,(map_one_ (lambda (varval) (list (car varval) ''*undefined*)) bindings) 
            ,@(map_one_ (lambda (varval) (list 'set! (car varval) (cadr varval))) bindings)
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

(define-syntax catch
    (lambda (handler . body)
        `(with-exception-handler ,handler (lambda () ,@body)))
)

(define-syntax cond 
    (lambda clauses
        (if (null? clauses)
            () 
            `(if ,(caar clauses) 
                (begin ,@(cdar clauses))
                (cond ,@(cdr clauses))))
    )
)

(define (assert-equal object value)
    (if (equal? object value) #t (error "test failed."))
)

(define-syntax assert-fails
    (lambda (expr) 
        `(assert-equal 'test-failed 
            (catch (lambda (err) 'test-failed) ,expr)
        ))
)

(define (reverse lst)
    (let loop ((remaining lst) (acc '()))
        (if (null? remaining)
            acc
            (loop (cdr remaining) (cons (car remaining) acc)))
    )   
)

(define (map-cars lists) (map_one_ car lists))
(define (map-cdrs lists) (map_one_ cdr lists))

(define (map proc . lists) 
    (let loop ((lists lists) (acc '()))    
        (if (null? (car lists))
            (reverse acc)
            (loop (map-cdrs lists) (cons (apply proc (map-cars lists)) acc))))
)
 
 ; TODO Proper definition of write and display.
(define (newline) (debug "\n"))
(define! display debug)
(define! write debug)

(define (boolean? stuff) (or (= stuff #t) (= stuff #f)))

(define! for-each map)