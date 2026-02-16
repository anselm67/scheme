
(load "src/scheme/macros.scm")

;; This is a bit contrieved because of the newline, sorry (!)
(assert-equal (string-append "\"hello\"" (make-string 1 #\newline))
    (let ((port (open-output-string)))
        (with-output-port
            port 
            (lambda () (debug "hello") (get-output-string))))
)

(assert-fails (let ((port (open-output-string)))
    (close-output-port port)
    (write-char port #\a))
)

(assert-equal '(1 2 3) (let ((port (open-input-string "(1 2 3)")))
    (with-input-port port (lambda () (read))))
)
