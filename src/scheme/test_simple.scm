;; Some scheme tests.

(load "src/scheme/macros.scm")

(define (assert-equal object value)
    (if (equal? object value) #t (error "test failed."))
)

(assert-equal 1 1)
(assert-equal '(1 2) (list 1 2))
