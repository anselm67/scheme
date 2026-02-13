
(define counter
    (let ((value 0))
        (lambda () (set! value (+ value 1)) value))
)
