;; Some scheme tests.

;; Testing some predicates.
(assert-equal #t (procedure? +))
(assert-equal #f (procedure? 1))
(assert-equal #t (procedure? (lambda () ())))

(assert-equal #t (closure? (lambda () ())))
(assert-equal #f (closure? 1))

;; Testing lists
(assert-equal '(3 2 1) (reverse '(1 2 3)))
(assert-equal '() (reverse '()))

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

;; Test apply.
(assert-equal 6 (apply + '(1 2 3)))
(assert-equal 6 (apply + 1 2 '(3)))
(assert-equal 1 (apply + 1 (list)))
(assert-equal 1 (apply + 1 ()))

;; Test error handling
(assert-equal 1 (catch (lambda (e) 1) (error "foo")))
(assert-equal 1 (catch (lambda (e) 2) 3 4 1))

