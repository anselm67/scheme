; macros.scm
; This file is bundled with the interpretor and defines all the well known macros
; and functions that aren't defined as primitives.

(define! cadr (lambda (l) (car (cdr l))))
(define! cddr (lambda (l) (cdr (cdr l))))
(define! caar (lambda (l) (car (car l))))
(define! cdar (lambda (l) (cdr (car l))))
(define! cadar (lambda (l) (car (cdr (car l)))))
(define! caddr (lambda (l) (car (cdr (cdr l)))))
(define! cddar (lambda (l) (cdr (cdr (car l)))))
(define! caddar (lambda (l) (car (cdr (cdr (car l))))))

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

(define-syntax let* 
    (lambda (bindings . body) 
        (if (null? bindings)
            `(begin ,@body)
            `(let ((,(caar bindings) ,(cadar bindings)))
                (let* ,(cdr bindings) ,@body))
        )
    )
)

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

(define-syntax and
    (lambda exprs
        (if (null? exprs)
            #t
            `(let ((first ,(car exprs)))
                (if (not first) first (and ,@(cdr exprs))))
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
            (if (eq? '=> (cadar clauses)) 
                `(let ((_test ,(caar clauses))
                       (_proc ,(caddar clauses)))
                    (if _test (_proc _test) (cond ,@(cdr clauses))))
                (if (eq? 'else (caar clauses))
                    `(begin ,@(cdar clauses))
                    `(let ((test_ ,(caar clauses)))
                        (if test_
                            (begin ,@(cdar clauses))
                            (cond ,@(cdr clauses)))))))
    )
)

(define (assert-equal object value)
    (if (equal? object value) #t (error "test failed."))
)

(define-syntax assert-fails
    (lambda (expr) 
        `(assert-equal 'test-failed 
            (catch (lambda (err) 'test-fail(let ((port (open-output-string)))
        (with-output-port
            port 
            (lambda () (write "hello\n") (get-output-string))))ed) ,expr)
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
(define (newline) (display "\n"))

(define (boolean? stuff) (or (eq? stuff #t) (eq? stuff #f)))

(define! for-each map)

(define (not x) (eq? #f x))

(define (assv obj alist) 
    (let loop ((lst alist))
        (if (null? lst) #f
            (if (eq? obj (caar lst))
                (car lst)
                (loop (cdr lst))))
    )
)

(define (memq obj lst)
    (if (null? lst)
        #f
        (if (eq? obj (car lst))
            lst
            (memq obj (cdr lst))))
)

(define-syntax case
    (lambda (expr . clauses)
        (if (null? clauses) 
            #f
            `(let ((value_ ,expr))
                (if (eq? 'else (quote ,(caar clauses)))
                    (begin ,@(cdar clauses))
                    (if (memq value_ (quote ,(caar clauses))) 
                        (begin ,@(cdar clauses))
                        (case value_ ,@(cdr clauses)))))
            )
    )
)

(define (zero? num) (= num 0))
(define (negative? num) (< num 0))
(define (positive? num) (> num 0))
(define (abs num) (if (negative? num) (- num) num))

(define-syntax do  
    (lambda (var-init-steps test-expr . body)
        `(let loop 
            ,(map (lambda (vis) (list (car vis) (cadr vis))) var-init-steps) 
            (if ,(car test-expr)
                ,(cadr test-expr)
                (begin ,@body (loop ,@(map (lambda (vis) (if (null? (cddr vis)) (car vis) (caddr vis))) var-init-steps))))
        )
    )
)

