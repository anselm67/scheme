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

(define (diff a b) 
    (cond ((and (null? a) (null? b)) #t)
          ((and (pair? a) (pair? b)) 
            (if (equal? (car a) (car b)) 
                (verbose-equal? (cdr a) (cdr b))
                (debug "car a: " (car a) "\ncar b: " (car b))))
            (else (debug "a: " (car a) "b: " (car b))))
)

