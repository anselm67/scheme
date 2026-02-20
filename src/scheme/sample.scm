
(load "src/scheme/macros.scm")

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

(define (cat filename)
    (let ((port (open-input-file filename)))
        (let loop ((ch (read-char port)))
                (unless (eof-object? ch) 
                    (debug ch)
                    (loop (read-char port)))))
)

(define (gc-death) 
    (let loop () (begin (load "src/scheme/macros.scm") (heap-stats) (loop)))
)
