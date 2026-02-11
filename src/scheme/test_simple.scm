;; Some scheme tests.

(load "src/scheme/macros.scm")

(define (assert-equal object value)
    (if (equal? object value) #t (error "test failed."))
)

(assert-equal 1 1)
(assert-equal '(1 2) (list 1 2))

;; Testing the let macro.
(assert-equal 1 (let ((x 1)) x))

;; Testing the or macro.
(assert-equal #f (or))
(assert-equal 1 (or 1 (error "should not be evaluated.")))
(assert-equal 1 (or (or 1)))

