;; Sample use of the async primitives.

(define (create-task name limit) 
    (do ((i 0 (+ i 1))) ((> i limit) i) (debug name " " i) (sleep 250))
)

(define (create-task-id count) 
    (let loop ((task-id 0) (tasks ()))
        (if (> task-id 10)
            (reverse tasks)
            (cons (string-append "task-" (number->string task-id))
                (loop (+ task-id 1) tasks))))
)

(define (show task-count) 
    (let* ((tasks (map (lambda (name) (create-task name 10)) (create-task-id task-count)))
           (handles (map (lambda (task) (spawn task)) tasks)))
            ;; Wait for all tasks to terminate.
            (sleep 2500)
            (map (lambda (handle) (join handle)) handles))
)