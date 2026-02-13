
(define (loop start end)
    (if (>= start end) end (loop (+ start 1) end))
)