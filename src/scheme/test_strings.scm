;; Tests for the string primitives.

(load "src/scheme/macros.scm")

(assert-equal #t (string? "hello"))
(assert-equal #f (string? 1))

(assert-equal "   " (make-string 3))
(assert-equal "aaa" (make-string 3 #\a))

(assert-equal "abc" (string #\a #\b #\c))

(assert-equal '(#\a #\b #\c) (string->list "abc"))
(assert-equal (string->list "abc") '(#\a #\b #\c))

(assert-equal 5 (string-length "abcde"))

(assert-equal #\a (string-ref "bba" 2))
(assert-equal "aba" (string-set! "aaa" 1 #\b))

(assert-equal #t (string=? "abcd" "abcd"))
(assert-equal #f (string=? "abc" "abcd"))

(assert-equal #t (string<? "abcd" "abcde"))
(assert-equal #t (string<=? "abcd" "abcd"))
(assert-equal #t (string>? "bbb" "aaa"))
(assert-equal #t (string>=? "abcd" "abcd"))

(assert-equal #t (string-ci<? "abcd" "ABCDE"))
(assert-equal #t (string-ci<=? "ABCD" "abcd"))
(assert-equal #t (string-ci>? "bbb" "AAA"))
(assert-equal #t (string-ci>=? "abcd" "ABCD"))

(assert-equal "abcd" (string-append "ab" "cd"))
(assert-equal "abcd" (substring "xxabcdyy" 2 6))
(assert-equal "abcd" (string-copy "abcd"))
(assert-equal "aaaa" (string-fill! "abcd" #\a))


