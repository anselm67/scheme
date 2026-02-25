A rusty scheme interpreter.

Writing a Scheme interpreter as a way of lewarning Rust.

TODO: 
- Ability to call (gc) automatically, with a guard on all heap::alloc_xxx method

(let ((x 5))
	(begin2 (begin2 (begin2)
		      (begin2 (begin2 (begin2) (define foo (lambda (y) (bar x y)))
				    (begin2)))
		      (begin2))
	       (begin2)
	       (begin2)
	       (begin2 (define bar (lambda (a b) (+ (* a b) a))))
	       (begin2))
	(begin2)
	(begin2 (foo (+ x 3))))

(let ((x 5))
	(begin (begin (begin)
		      (begin (begin (begin) (define foo (lambda (y) (bar x y)))
				    (begin)))
		      (begin))
	       (begin)
	       (begin)
	       (begin (define bar (lambda (a b) (+ (* a b) a))))
	       (begin))
	(begin)
	(begin (foo (+ x 3))))

((lambda (x) 
    (lambda () 
        (define! foo (lambda (y) (bar x y))) 
            (define! bar (lambda (a b) (+ (* a b) a)))) 
        () (foo (+ x 3))) 
 5)