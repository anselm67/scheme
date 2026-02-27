; macros.scm
; This file is bundled with the interpretor and defines all the well known macros
; and functions that aren't defined as primitives.
; Order of the definitions matters a *lot*, you've been warned!

(define! cadr (lambda (l) (car (cdr l))))
(define! cddr (lambda (l) (cdr (cdr l))))
(define! caar (lambda (l) (car (car l))))
(define! cdar (lambda (l) (cdr (car l))))
(define! cadar (lambda (l) (car (cdr (car l)))))
(define! caddr (lambda (l) (car (cdr (cdr l)))))
(define! cddar (lambda (l) (cdr (cdr (car l)))))
(define! caddar (lambda (l) (car (cdr (cdr (car l))))))

(define! eqv? eq?)

(define-syntax define 
    (lambda (name_and_params . body) 
        (if (symbol? name_and_params)
            `(define! ,name_and_params ,(car body))
            `(define! ,(car name_and_params) (lambda ,(cdr name_and_params) ,@body)))
    )
)

(define (not x) (eq? #f x))

(define (map_one_ fn list) 
    (if (null? list) 
        ()
        (cons (fn (car list)) (map_one_ fn (cdr list))))
)

(define named_let_
    (lambda (name bindings body) 
        `((letrec ((,name (lambda ,(map_one_ car bindings) ,@body))) ,name)
            ,@(map_one_ cadr bindings)) ) 
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
            (if (null? (cdr exprs))
                `,(car exprs)
                `(let ((first ,(car exprs)))
                    (if (not first) first (and ,@(cdr exprs)))))))
)

(define-syntax catch
    (lambda (handler . body)
        `(with-exception-handler ,handler (lambda () ,@body)))
)

; This simple version of begin is just enough to define cond.
; A better version follows...

(define-syntax begin 
    (lambda exprs 
        (if (null? exprs)
            `'*unbound*
            `((lambda () ,@exprs))))
)

(define-syntax cond
    (lambda clauses
        (if (null? clauses)
            () 
            (if (null? (cdar clauses))
                `,(caar clauses)
                (if (eq? '=> (cadar clauses)) 
                    `(let ((_test ,(caar clauses))
                        (_proc ,(caddar clauses)))
                        (if _test (_proc _test) (cond ,@(cdr clauses))))
                    (if (eq? 'else (caar clauses))
                        `(begin ,@(cdar clauses))
                        `(let ((test_ ,(caar clauses)))
                            (if test_
                                (begin ,@(cdar clauses))
                                (cond ,@(cdr clauses))))))))
    )
)

(define (flatten-begin expr)
  (cond ((null? expr) '())
        ;; If the element is a (begin ...), unwrap it and append its flattened contents
        ((and (pair? (car expr)) (eq? (car (car expr)) 'begin))
         (append (flatten-begin (cdr (car expr))) ; Flatten the inside of the begin
                 (flatten-begin (cdr expr))))     ; Continue with the rest of the list
        ;; If the element is just an empty (begin), skip it
        ((eq? (car expr) 'begin) 
         (flatten-begin (cdr expr)))
        ;; Otherwise, keep the element and move on
        (else 
         (cons (car expr) (flatten-begin (cdr expr))))))

(define-syntax begin
    (lambda exprs 
        (let ((body (flatten-begin exprs)))
            (cond ((null? body) ())
                ((pair? (cdr body)) `((lambda () ,@body)))
                (else `,(car body)))))
)

(define (assert-equal object value)
    (if (equal? object value) #t 
        (begin (display "Expected: ") (display object) (newline)
            (display "Actual: ") (display value) (newline)
            (error "test failed.")))
)

(define-syntax assert-fails
    (lambda (expr)
        `(assert-equal 'test-failed 
            (catch (lambda (err) 'test-failed) ,expr)))
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
 
(define (newline . port) 
    (if (null? port)
        (display "\n")
        (display "\n" (car port)))
)


(define (boolean? stuff) (or (eq? stuff #t) (eq? stuff #f)))

(define! for-each map)

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
(define (complex? num) (number? num))
(define (real? num) (number? num))
(define (rational? num) (number? num))
(define (exact? num) (integer? num))
(define (inexact? num) (not (integer? num)))
(define (even? n) (zero? (% n 2)))
(define (odd? n) (not (even? n)))
(define lcm (lambda args
    (if (null? args) 
        1
        (let ((a (car args)))
            (if (null? (cdr args))
                (abs a)
                (/ (abs (* a (cadr args))) (gcd a (cadr args)))))))
)
(define (abs num) (if (negative? num) (- num) num))
(define! remainder %)

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

(define (list-ref lst pos)
    (cond 
        ((< pos 0) (error "Invalid list index."))
        ((= pos 0) (car lst))
        (else (list-ref (cdr lst) (- pos 1)))))

(define (last lst) 
    (if (null? (cdr lst)) (car lst) (last (cdr lst)))
)

(define (member item lst) 
    (cond ((null? lst) #f)
        ((equal? item (car lst)) lst)
        (else (member item (cdr lst)))
    )
)

(define (memv item lst) 
    (cond ((null? lst) #f)
        ((equal? item (car lst)) lst)
        (else (memv item (cdr lst)))
    )
)

(define (memq item lst)
    (cond ((null? lst) #f)
        ((eq? item (car lst)) lst)
        (else (memq item (cdr lst))))
)

(define (assoc item lst) 
    (cond ((null? lst) #f)
        ((equal? item (caar lst)) (car lst))
        (else (assoc item (cdr lst))))
)

(define (assv item lst) 
    (cond ((null? lst) #f)
        ((eqv? item (caar lst)) (car lst))
        (else (assv item (cdr lst))))
)

(define (assq item lst) 
    (cond ((null? lst) #f)
        ((eq? item (caar lst)) (car lst))
        (else (assq item (cdr lst))))
)

(define (call-with-input-file filename proc)
    (let ((port (open-input-file filename)))
        (catch (lambda (err) (close-input-port port)) 
            (let ((result (proc port)))
                    (close-input-port port)
                    result))))

(define (call-with-output-file filename proc)
    (let ((port (open-output-file filename)))
        (catch (lambda (err) (debug "FAILED" err) (close-output-port port)) 
            (let ((result (proc port)))
                    (close-output-port port)
                    result))))

(define (make-promise expr)
    (let ((ready? #f)
          (result #f))
        (lambda ()
            (if ready? 
                result
                (begin (set! result (expr))
                    (set! ready? #t)
                    result))))
)
 
(define-syntax delay 
    (lambda (expr) 
        `(make-promise (lambda () ,expr))
    )
)

(define (force promise) (promise))
