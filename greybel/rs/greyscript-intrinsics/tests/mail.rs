//! MetaMail objects (`MetaMailIntrinsics`): every method waits out the 2-second cooldowns, reads the account again
//! before using it, and takes its mail IDs as a list.

use crate::support::mail::run;

#[test]
fn should_refuse_a_mail_method_within_two_seconds_of_the_last() {
    let code = r#"mail = mail_login("test@test.com", "test")
print mail.fetch.len
print mail.fetch
wait(1.9)
print mail.fetch
wait(2)
print mail.fetch.len"#;

    let (output, error) = run(code);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\nMetaMail: Wait 2 seconds before using mail again.\n\
         MetaMail: Wait 2 seconds before using mail again.\n1"
    );
}

#[test]
fn should_share_the_cooldown_between_mail_objects() {
    let code = r#"one = mail_login("test@test.com", "test")
two = mail_login("test@test.com", "test")
print one.fetch.len
print two.fetch"#;

    let (output, error) = run(code);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\nMetaMail: Wait 2 seconds before using mail again."
    );
}

#[test]
fn should_read_and_delete_lists_of_mail_ids_in_the_current_inbox() {
    let code = r#"admin = include_lib("admin.so")
admin.create_mail_account("neo@test.com", "matrix1")
mail = mail_login("test@test.com", "test")
other = mail_login("neo@test.com", "matrix1")
print mail.send("neo@test.com", "hi", "first message body")
wait(2)
print mail.send("neo@test.com", "again", "second")
wait(2)
inbox = other.fetch
print inbox.len
print inbox[0].indexOf("second") != null
ids = []
for summary in inbox
  ids.push(summary.split("MailID: ")[1].split(char(10))[0])
end for
wait(2)
read = other.read([ids[1], "nope"])
print read.len
print read[0].indexOf("first message body") != null
print read[1]
wait(2)
print other.read("nope")
print other.read([1])
wait(2)
print other.delete(["nope"])
wait(2)
print other.delete([ids[1]])
wait(2)
print other.fetch.len
wait(2)
print other.delete_all
wait(2)
print other.fetch.len"#;

    let (output, error) = run(code);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\n1\n2\n1\n2\n1\nMail not found\nnull\nnull\nnull\n1\n1\n1\n0"
    );
}
