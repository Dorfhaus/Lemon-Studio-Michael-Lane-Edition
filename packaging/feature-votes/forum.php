<?php
// SPDX-License-Identifier: GPL-3.0-or-later
// Shared anonymous identities, plain text, bounded pages. Called within API transactions.
const FORUM_OWNER='owner:community';
function forum_schema(PDO $db): void {
 $db->exec('CREATE TABLE IF NOT EXISTS forum_profiles(user TEXT PRIMARY KEY REFERENCES users(id),name TEXT NOT NULL,avatar TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS forum_threads(id INTEGER PRIMARY KEY,title TEXT NOT NULL,author TEXT NOT NULL REFERENCES users(id),created INTEGER NOT NULL,updated INTEGER NOT NULL,hidden INTEGER NOT NULL DEFAULT 0,locked INTEGER NOT NULL DEFAULT 0);
 CREATE TABLE IF NOT EXISTS forum_posts(id INTEGER PRIMARY KEY,thread INTEGER NOT NULL REFERENCES forum_threads(id),parent INTEGER REFERENCES forum_posts(id),author TEXT NOT NULL REFERENCES users(id),body TEXT NOT NULL,created INTEGER NOT NULL,hidden INTEGER NOT NULL DEFAULT 0);
 CREATE INDEX IF NOT EXISTS forum_thread_posts ON forum_posts(thread,id);
 CREATE TABLE IF NOT EXISTS forum_reactions(user TEXT NOT NULL REFERENCES users(id),post INTEGER NOT NULL REFERENCES forum_posts(id),value INTEGER NOT NULL CHECK(value IN(-1,1)),PRIMARY KEY(user,post));');
}
function forum_text($value,int $min,int $max): string {
 if(!is_string($value)||strlen($value)>$max*4||!preg_match('//u',$value)){throw new DomainException('Invalid forum text.',400);}
 $value=trim(str_replace(["\r\n","\r"],"\n",$value));$n=preg_match_all('/./us',$value);
 if($n<$min||$n>$max||preg_match('/[\p{C}<>]|https?:|www\./iu',str_replace(["\n","\t"],'',$value))){throw new DomainException('Use '.$min.'-'.$max.' characters, without links or markup.',400);}
 return $value;
}
function forum_alias(PDO $db,string $user): string {if($user===FORUM_OWNER){return 'Administrator';}return 'Member '.strtoupper(substr(hash_hmac('sha256','forum:'.$user,query($db,'SELECT v FROM settings WHERE k=?',['secret'])->fetchColumn()),0,10));}
function forum_profile(PDO $db,string $user): array {return query($db,'SELECT name,avatar FROM forum_profiles WHERE user=?',[$user])->fetch(PDO::FETCH_ASSOC)?:['name'=>'','avatar'=>''];}
function forum_identity(PDO $db,string $user): array {$p=forum_profile($db,$user);return ['author'=>$p['name']?:forum_alias($db,$user),'member'=>forum_alias($db,$user),'avatar'=>$p['avatar']];}
function forum_page(PDO $db,string $user,int $thread=0,int $offset=0,bool $owner=false): array {
 if($thread<0||$offset<0||$offset>1000000){throw new DomainException('Invalid forum page.',400);}
 $data=['threads'=>[],'posts'=>[],'topic'=>null,'total'=>0,'offset'=>$offset,'profile'=>forum_profile($db,$user)];
 if(!$thread){
  $where=$owner?'':' WHERE t.hidden=0';
  $data['total']=(int)query($db,'SELECT COUNT(*) FROM forum_threads t'.$where)->fetchColumn();
  $rows=query($db,'SELECT t.*,(SELECT COUNT(*) FROM forum_posts p WHERE p.thread=t.id AND p.hidden=0) AS posts FROM forum_threads t'.$where.' ORDER BY t.updated DESC,t.id DESC LIMIT 20 OFFSET '.$offset)->fetchAll(PDO::FETCH_ASSOC);
  foreach($rows as &$row){$row=array_merge($row,forum_identity($db,$row['author']));foreach(['id','created','updated','hidden','locked','posts'] as $k){$row[$k]=(int)$row[$k];}}$data['threads']=$rows;
 } else {
  $topic=query($db,'SELECT t.*,(SELECT COUNT(*) FROM forum_posts p WHERE p.thread=t.id AND p.hidden=0) AS posts FROM forum_threads t WHERE t.id=?'.($owner?'':' AND t.hidden=0'),[$thread])->fetch(PDO::FETCH_ASSOC);
  if(!$topic){throw new DomainException('This thread is no longer available.',404);}
  $topic=array_merge($topic,forum_identity($db,$topic['author']));foreach(['id','created','updated','hidden','locked','posts'] as $k){$topic[$k]=(int)$topic[$k];}$data['topic']=$topic;
  $where=' WHERE p.thread=?'.($owner?'':' AND p.hidden=0');
  $data['total']=(int)query($db,'SELECT COUNT(*) FROM forum_posts p'.$where,[$thread])->fetchColumn();
  $rows=query($db,'SELECT p.*,u.blocked,COALESCE((SELECT SUM(r.value=1) FROM forum_reactions r JOIN users v ON v.id=r.user WHERE r.post=p.id AND v.blocked=0),0) AS up,COALESCE((SELECT SUM(r.value=-1) FROM forum_reactions r JOIN users v ON v.id=r.user WHERE r.post=p.id AND v.blocked=0),0) AS down,COALESCE((SELECT value FROM forum_reactions WHERE post=p.id AND user=?),0) AS mine FROM forum_posts p JOIN users u ON u.id=p.author'.$where.' ORDER BY p.id LIMIT 20 OFFSET '.$offset,[$user,$thread])->fetchAll(PDO::FETCH_ASSOC);
  foreach($rows as &$row){$row=array_merge($row,forum_identity($db,$row['author']));foreach(['id','thread','created','hidden','blocked','up','down','mine'] as $k){$row[$k]=(int)$row[$k];}$row['parent']=$row['parent']===null?null:(int)$row['parent'];} $data['posts']=$rows;
 }
 return $data;
}
function forum_handle(PDO $db,array $in,string $user,string $peer): array {
 $action=$in['action'];$thread=$in['thread']??0;$offset=$in['offset']??0;
 if(!is_int($thread)||!is_int($offset)){throw new DomainException('Invalid forum page.',400);}
 if($action==='forum_list'){return forum_page($db,$user,$thread,$offset);}
 if(!$user){throw new DomainException('Connect before posting.',401);}
 if($action==='forum_profile'){
  $name=forum_text($in['name']??'',2,32);$avatar=$in['avatar']??'';
  if(str_contains($name,"\n")||!is_string($avatar)||($avatar!==''&&!preg_match('/^[a-f0-9]{6144}$/D',$avatar))){throw new DomainException('Invalid profile name or picture.',400);}
  limit_request($db,'forum-profile:'.$user,10,3600);
  query($db,'INSERT INTO forum_profiles VALUES(?,?,?) ON CONFLICT(user) DO UPDATE SET name=excluded.name,avatar=excluded.avatar',[$user,$name,$avatar]);
  return forum_page($db,$user,$thread,$offset);
 }
 if($action==='forum_create'){
  $title=forum_text($in['title']??'',4,100);$body=forum_text($in['body']??'',1,800);
  if(str_contains($title,"\n")){throw new DomainException('Use one line for the title.',400);}
  limit_request($db,'forum-thread-user:'.$user,5,86400);limit_request($db,'forum-thread-net:'.$peer,10,86400);
  if(query($db,'SELECT COUNT(*) FROM forum_threads')->fetchColumn()>=10000){throw new DomainException('Forum is full for now.',429);}
  query($db,'INSERT INTO forum_threads(title,author,created,updated) VALUES(?,?,?,?)',[$title,$user,time(),time()]);$thread=(int)$db->lastInsertId();
  query($db,'INSERT INTO forum_posts(thread,author,body,created) VALUES(?,?,?,?)',[$thread,$user,$body,time()]);
  return forum_page($db,$user,$thread);
 }
 $topic=query($db,'SELECT * FROM forum_threads WHERE id=? AND hidden=0',[$thread])->fetch(PDO::FETCH_ASSOC);
 if(!$topic){throw new DomainException('Thread unavailable.',404);}
 if($topic['locked']){throw new DomainException('This thread is closed. You can still read it.',403);}
 if($action==='forum_reply'){
  $body=forum_text($in['body']??'',1,800);$parent=$in['parent']??null;
  if($parent!==null && (!is_int($parent)||!query($db,'SELECT 1 FROM forum_posts WHERE id=? AND thread=? AND hidden=0',[$parent,$thread])->fetchColumn())){throw new DomainException('Reply target unavailable.',400);}
  limit_request($db,'forum-post-minute:'.$user,6,60);limit_request($db,'forum-post-day:'.$user,100,86400);limit_request($db,'forum-post-net:'.$peer,200,86400);
  if(query($db,'SELECT COUNT(*) FROM forum_posts WHERE thread=?',[$thread])->fetchColumn()>=1000){throw new DomainException('This thread is full. Start a new thread.',409);}
  query($db,'INSERT INTO forum_posts(thread,parent,author,body,created) VALUES(?,?,?,?,?)',[$thread,$parent,$user,$body,time()]);query($db,'UPDATE forum_threads SET updated=? WHERE id=?',[time(),$thread]);
  $count=(int)query($db,'SELECT COUNT(*) FROM forum_posts WHERE thread=? AND hidden=0',[$thread])->fetchColumn();$offset=intdiv(max(0,$count-1),20)*20;
 } elseif($action==='forum_react'){
  $post=$in['post']??null;$value=$in['value']??null;
  if(!is_int($post)||!in_array($value,[-1,0,1],true)){throw new DomainException('Invalid reaction.',400);}
  if(!query($db,'SELECT 1 FROM forum_posts WHERE id=? AND thread=? AND hidden=0',[$post,$thread])->fetchColumn()){throw new DomainException('Post unavailable.',404);}
  limit_request($db,'forum-react:'.$user,40,60);
  if($value===0){query($db,'DELETE FROM forum_reactions WHERE user=? AND post=?',[$user,$post]);}
  else{query($db,'INSERT INTO forum_reactions VALUES(?,?,?) ON CONFLICT(user,post) DO UPDATE SET value=excluded.value',[$user,$post,$value]);}
 }else{throw new DomainException('Invalid forum action.',400);}
 return forum_page($db,$user,$thread,$offset);
}
function forum_moderate(PDO $db,array $in): array {
 $action=$in['action'];$id=$in['id']??null;$value=$in['value']??null;
 if(!is_int($id)||$id<1||!is_bool($value)){throw new DomainException('Invalid forum moderation.',400);}
 $db->exec('BEGIN IMMEDIATE');
 try {
  if(in_array($action,['forum_hide_thread','forum_lock'],true)){
   if(!query($db,'SELECT 1 FROM forum_threads WHERE id=?',[$id])->fetchColumn()){throw new DomainException('Thread not found.',404);}
   query($db,'UPDATE forum_threads SET '.($action==='forum_lock'?'locked':'hidden').'=? WHERE id=?',[(int)$value,$id]);
  }elseif(in_array($action,['forum_hide_post','forum_block','forum_clear_profile'],true)){
   $author=query($db,'SELECT author FROM forum_posts WHERE id=?',[$id])->fetchColumn();if(!$author){throw new DomainException('Post not found.',404);}
   if($action==='forum_clear_profile'){query($db,'DELETE FROM forum_profiles WHERE user=?',[$author]);}
   elseif($action==='forum_block'){query($db,'UPDATE users SET blocked=? WHERE id=?',[(int)$value,$author]);}
   else{query($db,'UPDATE forum_posts SET hidden=? WHERE id=?',[(int)$value,$id]);}
  }else{throw new DomainException('Invalid forum moderation action.',400);}
  query($db,'INSERT INTO moderation_log(created,action,target,detail) VALUES(?,?,?,?)',[time(),$action,(string)$id,$value?'1':'0']);$db->exec('COMMIT');return admin_board($db);
 }catch(Throwable $e){$db->exec('ROLLBACK');throw $e;}
}
