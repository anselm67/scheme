;; Some scheme tests.

(load "src/scheme/macros.scm")

(assert-equal 1 1)
(assert-equal '(1 2) (list 1 2))

;; Testing the let macro.
(assert-equal 1 (let ((x 1)) x))

;; Testing the or macro.
(assert-equal #f (or))
(assert-equal 1 (or 1 (error "should not be evaluated.")))
(assert-equal 1 (or (or 1)))

;; Testing vectors
(assert-equal #f (vector? 1))
(assert-equal #t (vector? #(1)))
(assert-equal #(1 1 1) (make-vector 3 1))
(assert-equal #(1 2 3) (vector 1 2 3))
(assert-equal 3 (vector-length #(1 2 3)))
(assert-equal 3 (vector-ref #(1 2 3) 2))
(assert-equal 0 (let ((v #(1 2 3))) (vector-set! v 0 0) (vector-ref v 0)))
(assert-equal '(1 2 3) (vector->list #(1 2 3)))
(assert-equal #(1 2 3) (list->vector '(1 2 3)))
(assert-equal #(0 0 0) (let ((v #(1 2 3))) (vector-fill! v 0) v))


; (assert-equal 1 2)
