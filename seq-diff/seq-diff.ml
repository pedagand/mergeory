type token = char

type del_pos = { del_pos: int; del_len: int }

type ins_anchor =
  | Before
  | After
  | Both

(* All indices are between existing positions:
    Original sequence : .0.1.2.3.4.
    Insertion index   : 0 1 2 3 4 5 *)
type ins_pos = { ins_pos: int; ins_anchor: ins_anchor }

type diff_atom =
  | Transfer of del_pos list * ins_pos list
  | Insert of token array * ins_pos

type ins_tokens = { ins_tokens: token array; linked: bool }

type patched_token =
  | Alive of { ins_before: ins_tokens option; token: token; ins_after: ins_tokens option }
  | Deleted

type patched_seq = { mutable ins_head: ins_tokens option; tokens: patched_token array; mutable ins_tail: ins_tokens option }

let linearize_ins ?(only_unlinked = false) opt_ins =
  match opt_ins with
  | Some {ins_tokens; linked = false} -> ins_tokens
  | Some {ins_tokens} when not only_unlinked -> ins_tokens
  | _ -> Array.init 0 (fun _ -> failwith "empty")

let linearize ?(skip_link_check = false) (tokens: patched_token array): token array =
  if Array.length tokens > 0 && not skip_link_check then (
    (match tokens.(0) with
     | Alive {ins_before = Some {linked = true}} -> invalid_arg "Starts with insertion linked with previous node"
     | _ -> ());
    (match tokens.(Array.length tokens - 1) with
     | Alive {ins_after = Some {linked = true}} -> invalid_arg "Ends with insertion linked with next node"
     | _ -> ())
  );
  Array.concat (Array.fold_right (fun elt acc -> match elt with
      | Alive {ins_before; token; ins_after} ->
        linearize_ins ~only_unlinked:true ins_before :: Array.make 1 token :: linearize_ins ins_after :: acc
      | Deleted -> acc) tokens [])

let linearize_seq {ins_head; tokens; ins_tail}: token array =
  Array.concat [linearize_ins ins_head; linearize ~skip_link_check:true tokens; linearize_ins ~only_unlinked:true ins_tail]

let prepare_patch (tokens: token array) =
  { ins_head = None;
    tokens = Array.map (fun tok -> Alive {ins_before = None; token = tok; ins_after = None}) tokens;
    ins_tail = None}

let delete_tokens (tokens: patched_token array) {del_pos; del_len}: token array =
  let sub_array = Array.sub tokens del_pos del_len in
  for i = del_pos to del_pos + del_len - 1 do
    tokens.(i) <- Deleted
  done;
  linearize sub_array

let try_push_ins (dest: ins_tokens option) (ins_tokens: ins_tokens) =
  match dest with
  | None -> Some ins_tokens
  | Some prev_token when ins_tokens = prev_token -> dest
  | Some _ -> invalid_arg "incompatible insertions"

let append_tokens (dest: patched_token) (ins_tokens: ins_tokens) =
  match dest with
  | Alive {ins_before; token; ins_after = prev_ins_opt} ->
    Alive {ins_before; token; ins_after = try_push_ins prev_ins_opt ins_tokens}
  | Deleted -> invalid_arg "inserting on a deleted node"

let prepend_tokens (dest: patched_token) (ins_tokens: ins_tokens) =
  match dest with
  | Alive {ins_before = prev_ins_opt; token; ins_after} ->
    Alive {ins_before = try_push_ins prev_ins_opt ins_tokens; token; ins_after}
  | Deleted -> invalid_arg "inserting on a deleted node"

let rec insert_tokens (patched_seq: patched_seq) (ins_tokens: token array) ?(force_link = false) {ins_pos; ins_anchor} =
  match ins_anchor with
  | Before ->
    if ins_pos > 0 then
      patched_seq.tokens.(ins_pos - 1) <- append_tokens patched_seq.tokens.(ins_pos - 1) {ins_tokens; linked = force_link}
    else
      patched_seq.ins_head <- try_push_ins patched_seq.ins_head {ins_tokens; linked = force_link}
  | After ->
    if ins_pos < Array.length patched_seq.tokens then
      patched_seq.tokens.(ins_pos) <- prepend_tokens patched_seq.tokens.(ins_pos) {ins_tokens; linked = force_link}
    else
      patched_seq.ins_tail <- try_push_ins patched_seq.ins_tail {ins_tokens; linked = force_link}
  | Both ->
    insert_tokens patched_seq ins_tokens ~force_link:true {ins_pos; ins_anchor = Before};
    insert_tokens patched_seq ins_tokens ~force_link:true {ins_pos; ins_anchor = After}

let atom_patch (patched_seq: patched_seq) (d: diff_atom) =
  match d with
  | Transfer(del, ins) ->
    let captured_tokens = Option.get (List.fold_left (fun prev_capture del_pos ->
        let new_capture = delete_tokens patched_seq.tokens del_pos in
        Option.iter (fun prev_capture -> if new_capture <> prev_capture then invalid_arg "incompatible captures") prev_capture;
        Some new_capture) None del) in
    List.iter (insert_tokens patched_seq captured_tokens) ins
  | Insert(ins_tokens, ins_pos) -> insert_tokens patched_seq ins_tokens ins_pos

let patch (token_seq: token array) (diff: diff_atom list) : token array =
  let patched_seq = prepare_patch token_seq in
  List.iter (atom_patch patched_seq) diff;
  linearize_seq patched_seq

let seq_example = [|'A'; 'B'; 'C'; 'B'; 'C'; 'D'; 'E'; 'F'; 'G'|]
let diff_example = [Insert ([|'A'|], {ins_pos = 3; ins_anchor = After});
                    Transfer ([{del_pos = 0; del_len = 2}; {del_pos = 3; del_len = 1}],
                              [{ins_pos = 6; ins_anchor = Before}; {ins_pos = 9; ins_anchor = Both}])]

let print_seq (seq: token array) =
  Array.iter (Printf.printf "%c") seq;
  Printf.printf "\n"
;;
print_seq (patch seq_example diff_example);;
