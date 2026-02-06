;; Some scheme tests.

(let ((l1 (list 1 2))
      (l2 (list 3 4)))
    (assert-eq (cddr (append l1 l2) l2))
)

(assert-eq (append) ())
