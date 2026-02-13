
(load "src/scheme/macros.scm")

(define counter 
    (let ((value 0))
        (lambda () (set! value (+ value 1)) value)
    )
)

(assert-equal 1 (counter))
(assert-equal 2 (counter))