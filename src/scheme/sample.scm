
(define (tail start end) 
    (debug start ".." end)
    (if (< start end) 
        (tail (+ start 1) end)
        end))

(define (not-tail count) 
    (debug count)
    (not-tail (+ count 1))
    ;; Not reached!
    ()
)